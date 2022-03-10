#![allow(unused)]
use std::convert::Infallible;
use std::error::Error;
use std::ops::{ControlFlow, FromResidual, Try};
use std::sync::Mutex;
use vizia::*;

use crate::app_state::AppEvent;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct LogMessage {
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

pub struct LogBuf(Vec<LogMessage>);
#[must_use]
pub struct LogResult<T>(pub T, pub LogBuf);
pub struct LogResultLoaded<T, E>(LogResult<Result<T, E>>);

macro_rules! emit_log {
    ($cx:expr, $level:ident, $message:expr $(,$context:expr)* $(,)?) => {
        $cx.emit($crate::app_state::AppEvent::Log {
            message: ::std::sync::Mutex::new(Some(crate::logging::log!(
                $level,
                $message,
                $($context,)*
            )))
        });
    }
}

macro_rules! log {
    ($level:ident, $message:expr $(,$context:expr)* $(,)?) => {
        $crate::logging::LogMessage {
            level: $crate::logging::LogLevel::$level,
            source: format!("{}:{}", file!(), line!()),
            message: format!($message, $($context,)*),
        }
    }
}
pub(crate) use {emit_log, log};

impl LogBuf {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn push(&mut self, msg: LogMessage) {
        self.0.push(msg);
    }

    pub fn extend(&mut self, buf: LogBuf) {
        self.0.extend(buf.0.into_iter());
    }

    pub fn done<T>(self, result: T) -> LogResult<T> {
        LogResult::new(result, self)
    }
}

impl<T> LogResult<T> {
    pub fn new(t: T, buf: LogBuf) -> Self {
        Self(t, buf)
    }

    pub fn emit(self, cx: &mut Context) -> T {
        for msg in self.1 .0 {
            cx.emit(AppEvent::Log {
                message: Mutex::new(Some(msg)),
            });
        }

        self.0
    }

    pub fn emit_p(self, cx: &mut ContextProxy) -> T {
        for msg in self.1 .0 {
            cx.emit(AppEvent::Log {
                message: Mutex::new(Some(msg)),
            })
            .unwrap();
        }

        self.0
    }

    pub fn emit_buf(self, buf: &mut Vec<AppEvent>) -> T {
        for msg in self.1 .0 {
            buf.push(AppEvent::Log {
                message: Mutex::new(Some(msg)),
            });
        }

        self.0
    }

    pub fn offload(self, buf: &mut LogBuf) -> T {
        buf.extend(self.1);
        self.0
    }
}

impl<T, E> LogResult<Result<T, E>> {
    pub fn inload(mut self, buf: &mut LogBuf) -> LogResultLoaded<T, E> {
        buf.0.append(&mut self.1 .0);
        if self.0.is_err() {
            std::mem::swap(&mut self.1, buf);
        }
        LogResultLoaded(self)
    }
}

impl<T, E> Try for LogResultLoaded<T, E> {
    type Output = T;
    type Residual = LogResult<Result<Infallible, E>>;

    fn from_output(output: Self::Output) -> Self {
        Self(LogResult(Ok(output), LogBuf::new()))
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self.0 .0 {
            Ok(o) => ControlFlow::Continue(o),
            Err(e) => ControlFlow::Break(LogResult(Err(e), self.0 .1)),
        }
    }
}
impl<T, E> FromResidual for LogResultLoaded<T, E> {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        let e = residual.0.unwrap_err();
        Self(LogResult(Err(e), residual.1))
    }
}

pub trait ResultExt<T> {
    fn offload(self, level: LogLevel, buf: &mut LogBuf) -> Option<T>;
    fn emit(self, level: LogLevel, cx: &mut Context) -> Option<T>;
}

impl<T, E: Into<Box<dyn Error>>> ResultExt<T> for Result<T, E> {
    fn offload(self, level: LogLevel, buf: &mut LogBuf) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(s) => {
                let s = s.into();
                buf.push(LogMessage {
                    level,
                    source: s
                        .backtrace()
                        .map_or("unknown offload".to_string(), |bt| bt.to_string()),
                    message: s.to_string(),
                });
                None
            }
        }
    }

    fn emit(self, level: LogLevel, cx: &mut Context) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(s) => {
                let s = s.into();
                cx.emit(LogMessage {
                    level,
                    source: s
                        .backtrace()
                        .map_or("unknown emit".to_string(), |bt| bt.to_string()),
                    message: s.to_string(),
                });
                None
            }
        }
    }
}
