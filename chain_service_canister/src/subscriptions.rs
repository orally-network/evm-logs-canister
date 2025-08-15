use crate::state::read_state;

/// Build a combined address list and topics filter:
/// - Addresses: unique 0x-prefixed hex strings of active subscriptions for this chain
/// - Topics: union of only the first topic position (topic0) across active subscriptions; None if empty
pub fn get_active_addresses_and_topics() -> (Vec<String>, Option<Vec<Vec<String>>>) {
    read_state(|state| {
        let chain_id = state.config.chain_id;

        let mut addresses: Vec<String> = Vec::new();
        let mut topic0_union: Vec<String> = Vec::new();

        for subscription in state.subscriptions.values() {
            if subscription.chain_id != chain_id {
                continue;
            }
            if !matches!(subscription.status, crate::types::SubscriptionStatus::Active) {
                continue;
            }

            // Collect addresses as 0x-prefixed hex strings
            if let Some(addr) = &subscription.filter.address {
                let addr_str = format!("0x{}", hex::encode(addr.as_ref()));
                if !addresses.contains(&addr_str) {
                    addresses.push(addr_str);
                }
            }

            // Collect only topic0 (first position) union
            if let Some(filter_topics) = &subscription.filter.topics {
                if let Some(first_pos) = filter_topics.get(0) {
                    for t in first_pos {
                        let topic_str = format!("0x{}", hex::encode(t.as_ref()));
                        if !topic0_union.contains(&topic_str) {
                            topic0_union.push(topic_str);
                        }
                    }
                }
            }
        }

        let topics = if topic0_union.is_empty() {
            None
        } else {
            Some(vec![topic0_union])
        };

        (addresses, topics)
    })
}
