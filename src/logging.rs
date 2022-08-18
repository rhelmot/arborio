use arborio_state::data::app::AppEvent;
use arborio_state::data::ArborioRecord;
use arborio_utils::vizia::prelude::*;
use log::{set_logger, set_max_level, Level, LevelFilter, Log, Metadata, Record};
use std::sync::mpsc::{sync_channel, SyncSender};

pub fn setup_logger_thread(cx: &mut Context) {
    let (tx, rx) = sync_channel(1000);
    let logger = Box::leak(Box::new(ViziaLogger { pipe: tx }));
    set_logger(logger).unwrap();
    set_max_level(LevelFilter::Debug);

    cx.spawn(move |cx| loop {
        let message = rx.recv().unwrap();
        cx.emit(AppEvent::Log { message }).unwrap();
    });
}

struct ViziaLogger {
    pipe: SyncSender<ArborioRecord>,
}

impl Log for ViziaLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if metadata.level() == Level::Debug && !metadata.target().starts_with("arborio") {
            return false;
        }
        #[cfg(debug_assertions)]
        {
            metadata.level() <= Level::Debug
        }
        #[cfg(not(debug_assertions))]
        {
            metadata.level() <= Level::Info
        }
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            self.pipe
                .send(ArborioRecord {
                    level: record.level(),
                    message: format!("{}", record.args()),
                })
                .unwrap();
        }
    }

    fn flush(&self) {}
}
