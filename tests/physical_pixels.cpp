#include <QColor>
#include <QGuiApplication>
#include <QImage>
#include <QQuickItem>
#include <QQuickView>
#include <QJSValue>
#include <QTimer>
#include <QDebug>
#include <cmath>

// QML TestCase.grabImage crops in logical coordinates. Read the full, native
// framebuffer here so fractional-DPR tests cannot hide resampling artifacts.
int main(int argc, char **argv) {
    QGuiApplication app(argc, argv);
    if (argc != 2) return 2;
    QQuickView view;
    view.setSource(QUrl::fromLocalFile(QString::fromLocal8Bit(argv[1])));
    if (view.status() == QQuickView::Error) return 2;
    view.show();

    QTimer::singleShot(100, &app, [&]() {
        auto *wrapper = view.rootObject()->findChild<QQuickItem *>("windowIndicator");
        auto *indicator = view.rootObject()->findChild<QQuickItem *>("dictationIndicator");
        if (!wrapper || !indicator) { app.exit(2); return; }
        const qreal ratio = view.devicePixelRatio();
        view.rootObject()->setProperty("renderScale", ratio);
        int failures = 0;
        auto check = [&](bool ok, const QString &message) {
            if (!ok && failures++ < 8) qCritical().noquote() << message;
        };
        auto integer = [](qreal value) { return std::abs(value - std::round(value)) < 0.0001; };
        check(std::abs(indicator->property("devicePixelRatio").toDouble() - ratio) < 0.0001,
              "QML must receive the real window DPR");
        const auto grid = indicator->property("grid").value<QJSValue>().toVariant().toMap();
        check(grid["stepX"] == grid["stepY"], "Rows and columns must have identical pitch");

        for (const QPointF origin : {QPointF(12, 34), QPointF(12.3, 34.2), QPointF(-3.1, 8.7)}) {
            wrapper->setX(origin.x());
            wrapper->setY(origin.y());
            for (const QString state : {QString("recording"), QString("transcribing"), QString("streaming")}) {
                wrapper->setProperty("daemonState", state);
                QImage image = view.grabWindow();
                check(!image.isNull(), "Native framebuffer capture failed");
                if (image.isNull()) continue;
                check(image.width() == qRound(view.width() * ratio), "Capture must have physical-pixel resolution");
                const QPointF topLeft = indicator->mapToScene(QPointF(0, 0)) * ratio;
                check(integer(topLeft.x()) && integer(topLeft.y()), "Whole indicator origin must align to physical pixels");
                int commonSide = -1;
                const auto dots = indicator->property("cells").value<QJSValue>().toVariant().toList();
                check(dots.size() == 162, "Expected 162 dot rectangles");
                for (const auto &value : dots) {
                    const auto dot = value.toMap();
                    const QPointF position = topLeft + QPointF(dot["x"].toDouble(), dot["y"].toDouble());
                    const qreal side = dot["width"].toDouble();
                    check(integer(position.x()) && integer(position.y()) && integer(side), "Dot has a fractional physical edge");
                    check(std::abs(dot["height"].toDouble() - side) < 0.0001, "Dot is not square");
                    const int size = qRound(side);
                    if (commonSide < 0) commonSide = size;
                    check(size == commonSide && size >= 3, "Dots have irregular or thin-line dimensions");
                    const int x = qRound(position.x());
                    const int y = qRound(position.y());
                    const QColor accent = indicator->property("accent").value<QColor>();
                    const qreal opacity = dot["opacity"].toDouble();
                    const QColor expected(qRound(13 * (1 - opacity) + accent.red() * opacity),
                                          qRound(13 * (1 - opacity) + accent.green() * opacity),
                                          qRound(13 * (1 - opacity) + accent.blue() * opacity));
                    for (int dy = 0; dy < size; ++dy) {
                        for (int dx = 0; dx < size; ++dx) {
                            const QColor pixel = image.pixelColor(x + dx, y + dy);
                            check(std::abs(pixel.red() - expected.red()) <= 1
                                      && std::abs(pixel.green() - expected.green()) <= 1
                                      && std::abs(pixel.blue() - expected.blue()) <= 1,
                                  QString("DPR %1: dot pixel %2,%3 differs from its solid square").arg(ratio).arg(x + dx).arg(y + dy));
                        }
                    }
                    // Check a real black separator, not just equal QML dimensions.
                    if (dot["column"].toInt() < 26)
                        check(image.pixelColor(x + size, y) == QColor("#0D0D0D"), QString("Horizontal edge at %1,%2: %3").arg(x + size).arg(y).arg(image.pixelColor(x + size, y).name()));
                    if (dot["row"].toInt() < 5)
                        check(image.pixelColor(x, y + size) == QColor("#0D0D0D"), QString("Vertical edge at %1,%2: %3").arg(x).arg(y + size).arg(image.pixelColor(x, y + size).name()));
                }
                if (origin == QPointF(12, 34) && state == "recording")
                    image.save(QString("build/physical-dpr-%1.png").arg(ratio));
            }
        }
        if (!failures) qInfo().noquote() << QString("PASS physical framebuffer: DPR %1, 162 uniform square dots, 3 origins × 3 states").arg(ratio);
        app.exit(failures ? 1 : 0);
    });
    return app.exec();
}
