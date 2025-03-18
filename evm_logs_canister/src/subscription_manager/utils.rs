use evm_logs_types::{Event, Filter};

// Function to check if particular event matches specific filter
pub fn event_matches_filter(event: &Event, subscribers_filter: &Filter) -> bool {
  if subscribers_filter.address != event.log_entry.address {
    return false;
  }

  if let Some(filter_topics) = &subscribers_filter.topics {
    let event_topics = &event.log_entry.topics;

    if event_topics.len() < filter_topics.len() {
      return false;
    }

    return filter_topics.iter().enumerate().all(|(i, filter_topic_set)| {
      event_topics
        .get(i)
        .is_some_and(|event_topic| filter_topic_set.contains(event_topic))
    });
  }

  true
}

#[cfg(test)]
mod tests {
  use std::str::FromStr;

  use candid::Nat;
  use evm_logs_types::TopicsPosition;
  use evm_rpc_types::{Hex, Hex20, Hex32, LogEntry};

  use super::*;

  fn create_event(address: &str, topics: Option<Vec<&str>>) -> Event {
    Event {
      id: Nat::from(1u8),
      timestamp: 0,
      chain_id: 1,
      // data: Value::Text("test".to_string()),
      // address: address.to_string(),
      // topics: topics.map(|t| t.into_iter().map(|s| s.to_string()).collect()),
      // tx_hash: "".to_string(),
      log_entry: LogEntry {
        address: Hex20::from_str(address).unwrap(),
        topics: topics
          .unwrap_or_default()
          .into_iter()
          .filter_map(|s| Hex32::from_str(s).ok())
          .collect(),
        data: Hex::from(vec![]),
        block_number: None,
        transaction_hash: None,
        transaction_index: None,
        block_hash: None,
        log_index: None,
        removed: false,
      },
    }
  }

  fn create_filter(address: &str, topics: Option<Vec<Vec<&str>>>) -> Filter {
    Filter {
      address: Hex20::from_str(address).unwrap(),
      topics: topics.map(|ts| {
        ts.into_iter()
          .map(|topic_set| {
            topic_set
              .into_iter()
              .filter_map(|s| Hex32::from_str(s).ok())
              .collect::<TopicsPosition>()
          })
          .collect()
      }),
    }
  }

  const ADDR1_HEX20: &str = "0xd42AcA6E135D1dae6317e776F7EB96Eb91b8eb91";
  const ADDR2_HEX20: &str = "0xDA2efffa45cf5D960209aA0921Cf42a4a2a085cf";
  const ADDR3_HEX20: &str = "0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97";
  const ADDR4_HEX20: &str = "0xa2DA709980Effc0fA9413efEc7f5F86a849eCb93";
  const TOPIC1_HEX32: &str = "0x895950522ad88866c40789c503f088b8d88beb8de46cbb2c2329b3e968460492";
  const TOPIC2_HEX32: &str = "0xa213165f23ed89a7627d7a82973d251d654614d33104f7aabd1ed4d40ecebbea";
  const TOPIC3_HEX32: &str = "0xf8bbdcc71146cb4ca500980f5a60a18d7ce1860f1d22f08a2d25f3dbb202e42e";
  const TOPIC4_HEX32: &str = "0x13b2fce0c601a939e04b82c993f795d4df1335747782cacdaff2d3b71b576002";
  const ADDR1_HEX20_LOWER: &str = "0xd42aca6e135d1dae6317e776f7eb96eb91b8eb91";
  const ADDR2_HEX20_LOWER: &str = "0xda2efffa45cf5d960209aa0921cf42a4a2a085cf";
  const ADDR3_HEX20_LOWER: &str = "0x4838b106fce9647bdf1e7877bf73ce8b0bad5f97";
  const ADDR4_HEX20_LOWER: &str = "0xa2da709980effc0fa9413efec7f5f86a849ecb93";
  const TOPIC1_HEX32_LOWER: &str = "0x895950522ad88866c40789c503f088b8d88beb8de46cbb2c2329b3e968460492";
  const TOPIC2_HEX32_LOWER: &str = "0xa213165f23ed89a7627d7a82973d251d654614d33104f7aabd1ed4d40ecebbea";
  const TOPIC3_HEX32_LOWER: &str = "0xf8bbdcc71146cb4ca500980f5a60a18d7ce1860f1d22f08a2d25f3dbb202e42e";
  const TOPIC4_HEX32_LOWER: &str = "0x13b2fce0c601a939e04b82c993f795d4df1335747782cacdaff2d3b71b576002";

  #[test]
  fn test_event_matches_filter_address_only_match() {
    let event = create_event(ADDR1_HEX20, None);
    let filter = create_filter(ADDR1_HEX20_LOWER, None);

    assert!(event_matches_filter(&event, &filter));
  }

  #[test]
  fn test_event_matches_filter_address_only_no_match() {
    let event = create_event(ADDR2_HEX20, None);
    let filter = create_filter(ADDR1_HEX20_LOWER, None);

    assert!(!event_matches_filter(&event, &filter));
  }

  #[test]
  fn test_event_matches_filter_topics_match() {
    let event = create_event(ADDR1_HEX20, Some(vec![TOPIC1_HEX32, TOPIC2_HEX32]));
    let filter = create_filter(ADDR1_HEX20_LOWER, Some(vec![vec![TOPIC1_HEX32], vec![TOPIC2_HEX32]]));

    assert!(event_matches_filter(&event, &filter));
  }

  #[test]
  fn test_event_matches_filter_topics_no_match() {
    let event = create_event(ADDR1_HEX20, Some(vec![TOPIC1_HEX32, TOPIC3_HEX32]));
    let filter = create_filter(ADDR1_HEX20_LOWER, Some(vec![vec![TOPIC1_HEX32], vec![TOPIC2_HEX32]]));

    assert!(!event_matches_filter(&event, &filter));
  }

  #[test]
  fn test_event_matches_filter_topics_partial_match() {
    let event = create_event(ADDR1_HEX20, Some(vec![TOPIC1_HEX32]));
    let filter = create_filter(ADDR1_HEX20_LOWER, Some(vec![vec![TOPIC1_HEX32, TOPIC2_HEX32]]));

    assert!(event_matches_filter(&event, &filter));
  }

  #[test]
  fn test_event_matches_filter_too_few_event_topics() {
    let event = create_event(ADDR1_HEX20, Some(vec![TOPIC1_HEX32]));
    let filter = create_filter(ADDR1_HEX20_LOWER, Some(vec![vec![TOPIC1_HEX32], vec![TOPIC2_HEX32]]));

    assert!(!event_matches_filter(&event, &filter));
  }

  #[test]
  fn test_event_matches_filter_no_filter_topics() {
    let event = create_event(ADDR1_HEX20, Some(vec![TOPIC1_HEX32, TOPIC2_HEX32]));
    let filter = create_filter(ADDR1_HEX20_LOWER, None);

    assert!(event_matches_filter(&event, &filter));
  }

  #[test]
  fn test_event_matches_filter_no_event_topics() {
    let event = create_event(ADDR1_HEX20, None);
    let filter = create_filter(ADDR1_HEX20_LOWER, Some(vec![vec![TOPIC1_HEX32], vec![TOPIC2_HEX32]]));

    assert!(!event_matches_filter(&event, &filter));
  }
}
