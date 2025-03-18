use std::{cell::RefCell, collections::HashMap, io};

use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct Metric {
  pub name: String,
  pub help: String,
  pub typ: String,
  pub label_names: Vec<String>,
  pub inner: HashMap<Vec<String>, Vec<Inner>>,
}

#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct Inner {
  pub value: u128,
  pub label_values: Vec<String>,
}

#[allow(unused)]
impl Inner {
  pub fn new(value: u128, label_values: Vec<String>) -> Self {
    Self { value, label_values }
  }

  pub fn inc(&mut self) {
    self.value = self.value.saturating_add(1);
  }

  pub fn dec(&mut self) {
    self.value = self.value.saturating_sub(1);
  }

  pub fn set(&mut self, value: u128) {
    self.value = value;
  }

  pub fn get(&self) -> u128 {
    self.value
  }

  pub fn inc_by(&mut self, value: u128) {
    self.value = self.value.saturating_add(value);
  }

  pub fn dec_by(&mut self, value: u128) {
    self.value = self.value.saturating_sub(value);
  }
}

#[allow(unused)]
impl Metric {
  pub fn new(name: &str, help: &str, typ: &str, label_names: &[&str]) -> Self {
    let label_names = label_names.iter().map(|s| s.to_string()).collect();
    Self {
      name: name.to_string(),
      help: help.to_string(),
      typ: typ.to_string(),
      label_names,
      inner: HashMap::new(),
    }
  }

  pub fn inc(&mut self) {
    self.inc_by(1);
  }

  pub fn inc_by(&mut self, val: u128) {
    self
      .inner
      .entry(vec![])
      .or_insert_with(|| vec![Inner::new(val, vec![])])
      .first_mut()
      .expect("should be here")
      .inc_by(val);
  }

  pub fn dec(&mut self) {
    self.dec_by(1);
  }

  pub fn dec_by(&mut self, val: u128) {
    self
      .inner
      .entry(vec![])
      .or_insert_with(|| vec![Inner::new(0, vec![])])
      .first_mut()
      .expect("should be here")
      .dec_by(val);
  }

  pub fn set(&mut self, value: u128) {
    self
      .inner
      .entry(vec![])
      .or_insert_with(|| vec![Inner::new(value, vec![])])
      .first_mut()
      .expect("should be here")
      .set(value);
  }

  pub fn get(&self) -> u128 {
    self
      .inner
      .get(&vec![])
      .map(|inner_vec| inner_vec.first().unwrap().value)
      .unwrap_or(0)
  }

  fn check_label_values(&self, label_values: &[String]) {
    if label_values.len() != self.label_names.len() {
      panic!(
        "Invalid number of labels. Expected {}, got {}",
        self.label_names.len(),
        label_values.len()
      )
    }
  }

  pub fn with_label_values(&mut self, label_values: Vec<String>) -> &mut Inner {
    self.check_label_values(&label_values);

    self
      .inner
      .entry(label_values.clone())
      .or_insert_with(|| vec![Inner::new(0, label_values)])
      .first_mut()
      .unwrap()
  }

  fn encode_header<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
    writeln!(w, "# HELP {} {}", self.name, self.help)?;
    writeln!(w, "# TYPE {} {}", self.name, self.typ)
  }

  fn encode_value<W: io::Write>(&self, w: &mut W) -> io::Result<()> {
    for (label_values, inner_vec) in self.inner.iter() {
      if label_values.len() != self.label_names.len() {
        continue;
      }

      if label_values.is_empty() {
        writeln!(w, "{} {}", self.name, inner_vec.first().unwrap().value)?;
        return Ok(());
      }

      for inner in inner_vec {
        let labels = self
          .label_names
          .iter()
          .zip(label_values.iter())
          .map(|(label_name, label_value)| format!("{}=\"{}\"", label_name, label_value))
          .collect::<Vec<String>>()
          .join(",");

        writeln!(w, "{}{{{}}} {}", self.name, labels, inner.value)?;
      }
    }

    Ok(())
  }

  pub fn encode<W: io::Write>(&self, w: &mut W) -> std::io::Result<()> {
    if self.inner.is_empty() {
      return Ok(());
    }

    self.encode_header(w)?;
    self.encode_value(w)
  }
}

#[allow(non_snake_case)]
#[derive(CandidType, Clone, Debug, Default, Deserialize, Serialize)]
pub struct Metrics {
  pub CUSTOM_FEEDS: Metric,
  pub DEFAULT_FEEDS: Metric,
  pub GET_ASSET_DATA_CALLS: Metric,
  pub SUCCESSFUL_GET_ASSET_DATA_CALLS: Metric,
  pub GET_ASSET_DATA_WITH_PROOF_CALLS: Metric,
  pub SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS: Metric,
  pub FALLBACK_XRC_CALLS: Metric,
  pub SUCCESSFUL_FALLBACK_XRC_CALLS: Metric,
  pub XRC_CALLS: Metric,
  pub SUCCESSFUL_XRC_CALLS: Metric,
  pub CYCLES: Metric,
}

thread_local! {
    pub static METRICS: RefCell<Metrics> = RefCell::new(Metrics{
        CUSTOM_FEEDS: Metric::new(
                "custom_feeds",
                "Number of custom feeds",
                "gauge",
                &[],
            ),
        DEFAULT_FEEDS: Metric::new(
                "default_feeds",
                "Number of default feeds",
                "gauge",
                &[],
            ),
        GET_ASSET_DATA_CALLS: Metric::new(
                "get_asset_data_calls",
                "Number of get_asset_data calls",
                "counter",
                &["feed"],
            ),
        SUCCESSFUL_GET_ASSET_DATA_CALLS: Metric::new(
                "successful_get_asset_data_calls",
                "Number of successfully returned get_asset_data calls",
                "counter",
                &["feed"],
            ),
        GET_ASSET_DATA_WITH_PROOF_CALLS: Metric::new(
                "get_asset_data_with_proof_calls",
                "Number of get_asset_data_with_proof calls",
                "counter",
                &["feed"],
            ),
        SUCCESSFUL_GET_ASSET_DATA_WITH_PROOF_CALLS: Metric::new(
                "successful_get_asset_data_with_proof_calls",
                "Number of successfully returned get_asset_data_with_proof calls",
                "counter",
                &["feed"],
            ),
        FALLBACK_XRC_CALLS: Metric::new(
                "fallback_xrc_calls",
                "Number of fallback_xrc calls",
                "counter",
                &[],
            ),
        SUCCESSFUL_FALLBACK_XRC_CALLS: Metric::new(
                "fallback_xrc_calls",
                "Number of fallback_xrc calls",
                "counter",
                &[],
            ),
        XRC_CALLS: Metric::new(
                "xrc_calls",
                "Number of xrc calls",
                "counter",
                &[],
            ),
        SUCCESSFUL_XRC_CALLS: Metric::new(
                "successful_xrc_calls",
                "Number of successfully returned xrc calls",
                "counter",
                &[],
            ),
        CYCLES: Metric::new(
                "cycles",
                "Number of cycles",
                "gauge",
                &[],
            ),
    });
}

#[macro_export]
macro_rules! metrics {
    ( inc $metric:ident ) => {
        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.inc());
    };

    ( inc_by $metric:ident, $val:ident ) => {
        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.inc_by($val as u128));
    };

    ( inc $metric:ident, $($labels:expr),+) => {{
        let lbls: Vec<String> = vec![$(format!("{}", $labels)),+];

        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(lbls).inc());
        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(vec!["all".to_string()]).inc());
    }};


    ( dec $metric:ident ) => {
        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.dec());
    };

    ( dec_by $metric:ident, $val:ident ) => {
        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.dec_by($val as u128));
    };

    ( dec $metric:ident, $($labels:expr),+) => {
        let lbls: Vec<String> = vec![$(format!("{}", $labels)),+];

        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(lbls).dec());
        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(vec!["all".to_string()]).dec());
    };


    ( get $metric:ident ) => {
        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.get())
    };

    ( get $metric:ident, $($labels:expr),+) => {
        {
            let lbls: Vec<String> = vec![$(format!("{}", $labels)),+];

            $crate::internals::log_metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(lbls).get())
        }
    };

    ( timer $metric:ident, $($labels:expr),+) => {
        let lbls: Vec<String> = vec![$(format!("{}", $labels)),+];

        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(lbls).start_timer());
    };

    ( timer $metric:ident) => {
        $crate::internals::log_metrics::METRICS.with(|m| m.borrow_mut().$metric.start_timer())
    };

    ( timer observe $timer:ident) => {
        $timer.observe_duration()
    };

    ( timer discard $timer:ident) => {
        $timer.stop_and_discard()
    };

    ( set $metric:ident, $val:expr ) => {
        $crate::internals::log_metrics::METRICS.with(|m| m.borrow_mut().$metric.set($val as u128));
    };

    ( set $metric:ident, $val:expr, $($labels:expr),+) => {
        let lbls: Vec<String> = vec![$(format!("{}", $labels)),+];

        let prev_val = $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(lbls.clone()).get());

        let diff = $val as i128 - prev_val as i128;


        $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(lbls).set($val as u128));
        if diff < prev_val as i128 {
            $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(vec!["all".to_string()]).dec_by(diff as u128));
        } else {
            $crate::internals::metrics::METRICS.with(|m| m.borrow_mut().$metric.with_label_values(vec!["all".to_string()]).inc_by(diff as u128));
        }
    };
}
