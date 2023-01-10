use arborio_state::data::app::AppEvent;
use arborio_state::data::ArborioRecord;
use arborio_utils::vizia::prelude::*;
use log::{set_logger, set_max_level, Level, LevelFilter, Log, Metadata, Record};
use std::sync::mpsc::{sync_channel, SyncSender};

pub fn setup_logger_thread(cx: &mut Context) {
    let (tx, rx) = sync_channel(1000);
    let logger_vizia = ViziaLogger { pipe: tx };
    set_max_level(LevelFilter::Debug);
    let logger_stderr = env_logger::builder()
        .filter_level(LevelFilter::Info)
        .build();
    let logger = multi_log::MultiLogger::new(vec![Box::new(logger_vizia), Box::new(logger_stderr)]);
    set_logger(Box::leak(Box::new(logger))).unwrap();

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
        if metadata.level() >= Level::Warn && metadata.target() == "cosmic_text::font::fallback" {
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
