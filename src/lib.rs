//! A log4rs appender which routes logging events to dynamically created sub-appenders.
//!
//! For example, you may want to direct output to different directories based on a "job ID" stored
//! in the MDC:
//!
//! ```yaml
//! appenders:
//!   job:
//!     kind: routing
//!     router:
//!       kind: pattern
//!       pattern:
//!         kind: file
//!         path: "log/jobs/${mdc(job_id)}/output.log"
//!     cache:
//!       idle_timeout: 30 seconds
//! loggers:
//!   server::job_runner:
//!     appenders:
//!     - job
//! ```
//!
//! ```ignore
//! #[macro_use]
//! extern crate log;
//! extern crate log_mdc;
//!
//! # fn generate_job_id() -> String { "foobar".to_owned() }
//! # fn main() {
//! let job_id = generate_job_id();
//! log_mdc::insert("job_id", job_id);
//!
//! info!("Starting job");
//! # }
//! ```
#![doc(html_root_url="https://sfackler.github.io/log4rs-routing-appender/doc/v0.2.0")]
#![warn(missing_docs)]
extern crate antidote;
extern crate linked_hash_map;
extern crate log;
extern crate log4rs;

#[cfg(feature = "humantime")]
extern crate humantime;
#[cfg(feature = "log-mdc")]
extern crate log_mdc;
#[cfg(feature = "serde")]
extern crate serde;
#[cfg(feature = "serde-value")]
extern crate serde_value;
#[cfg(feature = "ordered-float")]
extern crate ordered_float;

#[cfg(feature = "serde_derive")]
#[macro_use]
extern crate serde_derive;

use antidote::Mutex;
use log::LogRecord;
use log4rs::append::Append;
use std::error::Error;
use std::fmt;
use std::time::Duration;

#[cfg(feature = "file")]
use log4rs::file::{Deserialize, Deserializers};
#[cfg(feature = "file")]
use serde::de::{self, Deserialize as SerdeDeserialize};
#[cfg(feature = "file")]
use serde_value::Value;
#[cfg(feature = "file")]
use std::collections::BTreeMap;

use route::{Cache, Route};

pub mod route;

/// Configuration for the `RoutingAppender`.
#[cfg(feature = "file")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingAppenderConfig {
    router: RouterConfig,
    #[serde(default)]
    cache: CacheConfig,
}

#[cfg(feature = "file")]
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct CacheConfig {
    #[serde(deserialize_with = "de_duration", default)]
    idle_timeout: Option<Duration>,
}


/// Registers the following mappings:
///
/// * Appenders
///     * "routing" -> `RoutingAppenderDeserializer`
/// * Routers
///     * "pattern" -> `PatternAppenderDeserializer`
///         * Requires the `pattern-router` feature (enabled by default).
#[cfg(feature = "file")]
pub fn register(d: &mut Deserializers) {
    d.insert("routing", RoutingAppenderDeserializer);

    #[cfg(feature = "pattern-router")]
    d.insert("pattern", route::pattern::PatternRouterDeserializer);
}

/// An appender which routes log events to dynamically constructed sub-appenders.
pub struct RoutingAppender {
    router: Box<Route>,
    cache: Mutex<Cache>,
}

impl fmt::Debug for RoutingAppender {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("RoutingAppender")
            .field("router", &self.router)
            .finish()
    }
}

impl Append for RoutingAppender {
    fn append(&self, record: &LogRecord) -> Result<(), Box<Error + Sync + Send>> {
        let appender = self.router.route(record, &mut self.cache.lock())?;
        appender.appender().append(record)
    }
}

impl RoutingAppender {
    /// Creates a new `RoutingAppender` builder.
    pub fn builder() -> RoutingAppenderBuilder {
        RoutingAppenderBuilder { idle_timeout: Duration::from_secs(2 * 60) }
    }
}

/// A builder for `RoutingAppender`s.
pub struct RoutingAppenderBuilder {
    idle_timeout: Duration,
}

impl RoutingAppenderBuilder {
    /// Sets the duration after which an appender that has not been used will be removed from the
    /// cache.
    ///
    /// Defaults to 2 minutes.
    pub fn idle_timeout(mut self, idle_timeout: Duration) -> RoutingAppenderBuilder {
        self.idle_timeout = idle_timeout;
        self
    }

    /// Consumes the builder, producing a `RoutingAppender`.
    pub fn build(self, router: Box<Route>) -> RoutingAppender {
        RoutingAppender {
            router: router,
            cache: Mutex::new(Cache::new(self.idle_timeout)),
        }
    }
}

/// A deserializer for the `RoutingAppender`.
///
/// # Configuration
///
/// ```yaml
/// kind: routing
///
/// # The router used to determine the appender to use for a log event.
/// # Required.
/// router:
///   kind: pattern
///   pattern:
///     kind: file
///     path: "log/${mdc(job_id)}.log"
///
/// # Configuration of the cache of appenders generated by the router.
/// cache:
///
///   # The duration that a cached appender has been unused after which it
///   # will be disposed of. Defaults to 2 minutes.
///   idle_timeout: 2 minutes
/// ```
#[cfg(feature = "file")]
pub struct RoutingAppenderDeserializer;

#[cfg(feature = "file")]
impl Deserialize for RoutingAppenderDeserializer {
    type Trait = Append;
    type Config = RoutingAppenderConfig;

    fn deserialize(&self,
                   config: RoutingAppenderConfig,
                   deserializers: &Deserializers)
                   -> Result<Box<Append>, Box<Error + Sync + Send>> {
        let mut builder = RoutingAppender::builder();
        if let Some(idle_timeout) = config.cache.idle_timeout {
            builder = builder.idle_timeout(idle_timeout);
        }
        let router = deserializers.deserialize(&config.router.kind, config.router.config)?;
        Ok(Box::new(builder.build(router)))
    }
}

#[derive(PartialEq, Eq, Debug)]
#[cfg(feature = "file")]
struct RouterConfig {
    kind: String,
    config: Value,
}

#[cfg(feature = "file")]
impl<'de> de::Deserialize<'de> for RouterConfig {
    fn deserialize<D>(d: D) -> Result<RouterConfig, D::Error>
        where D: de::Deserializer<'de>
    {
        let mut map = BTreeMap::<Value, Value>::deserialize(d)?;

        let kind = match map.remove(&Value::String("kind".to_owned())) {
            Some(kind) => kind.deserialize_into().map_err(|e| e.to_error())?,
            None => return Err(de::Error::missing_field("kind")),
        };

        Ok(RouterConfig {
            kind: kind,
            config: Value::Map(map),
        })
    }
}

#[cfg(feature = "file")]
fn de_duration<'de, D>(d: D) -> Result<Option<Duration>, D::Error>
    where D: de::Deserializer<'de>
{
    struct S(Duration);

    impl<'de2> de::Deserialize<'de2> for S {
        fn deserialize<D>(d: D) -> Result<S, D::Error>
            where D: de::Deserializer<'de2>
        {
            struct V;

            impl<'de3> de::Visitor<'de3> for V {
                type Value = S;

                fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                    fmt.write_str("a duration")
                }

                fn visit_str<E>(self, v: &str) -> Result<S, E>
                    where E: de::Error
                {
                    humantime::parse_duration(v)
                        .map(S)
                        .map_err(|e| E::custom(&e.to_string()))
                }
            }

            d.deserialize_str(V)
        }
    }

    Option::<S>::deserialize(d).map(|d| d.map(|d| d.0))
}

trait CacheInner {
    fn new(expiration: Duration) -> Cache;
}

trait AppenderInner {
    fn appender(&self) -> &Append;
}
