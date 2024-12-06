use std::collections::{HashMap, HashSet};
use candid::Nat;
use evm_logs_types::Filter;

const MAX_TOPICS: usize = 4;

type TopicsPosition = Vec<String>;

pub struct FilterManager {
    topics: Vec<HashSet<String>>,               
    topic_counts: Vec<HashMap<String, Nat>>,
    addresses: HashMap<String, Nat>,    
    subscriptions_accept_any_topic_at_position: Vec<Nat>, // TODO HashMap<Address, arr[4]>
    total_subscriptions: Nat,                   
}

impl FilterManager {
    pub fn new() -> Self {
        FilterManager {
            topics: vec![HashSet::new(); MAX_TOPICS],
            topic_counts: vec![HashMap::new(); MAX_TOPICS],
            addresses: HashMap::new(),
            subscriptions_accept_any_topic_at_position: vec![Nat::from(0u32); MAX_TOPICS],
            total_subscriptions: Nat::from(0u32),
        }
    }

    /// Adds a new filter (subscription) to the manager, updating counts for addresses and topics.
    pub fn add_filter(&mut self, filter: &Filter) {
        self.total_subscriptions += Nat::from(1u32);
    
        for address in &filter.addresses {
            *self.addresses.entry(address.clone()).or_insert(Nat::from(0u32)) += Nat::from(1u32);
        }

        if let Some(filter_topics) = &filter.topics {
            for (i, topics_at_pos) in filter_topics.iter().enumerate() {
                if topics_at_pos.is_empty() {
                    // Position will accept any topic if empty
                    self.subscriptions_accept_any_topic_at_position[i] += Nat::from(1u32);
                }
                else {
                    for topic in topics_at_pos {
                        self.topics[i].insert(topic.clone());
                        *self.topic_counts[i].entry(topic.clone()).or_insert(Nat::from(0u32)) += Nat::from(1u32);
                    }
                }
            }

            // if filter has less than 4 topics than the rest of positions will accept any topic
            for i in filter_topics.len()..MAX_TOPICS {
                self.subscriptions_accept_any_topic_at_position[i] += Nat::from(1u32);
            }
        }
        else {
            // No topics specified => all positions accept any topic.
            for i in 0..MAX_TOPICS {
                self.subscriptions_accept_any_topic_at_position[i] += Nat::from(1u32);
            }        
        }
        ic_cdk::println!("Bit Mask after filter_add: {:?}", self.subscriptions_accept_any_topic_at_position);
        ic_cdk::println!("Topics after filter_add: {:?}", self.topics);
    }
    
    /// Removes a filter (subscription) from the manager, decrementing counts accordingly.
    pub fn remove_filter(&mut self, filter: &Filter) {
        if self.total_subscriptions > 0u32 {
            self.total_subscriptions -= Nat::from(1u32);
        }

        for address in &filter.addresses {
            if let Some(count) = self.addresses.get_mut(address) {
                *count -= Nat::from(1u32);
                if *count == 0u32 {
                    self.addresses.remove(address);
                }
            }
        }

        if let Some(filter_topics) = &filter.topics {
            ic_cdk::println!("in remove_filter, topics: {:?}", &filter.topics); // Some([["222"]])
            for (i, topics_at_pos) in filter_topics.iter().enumerate() {
                if topics_at_pos.is_empty() {
                    if self.subscriptions_accept_any_topic_at_position.len() > i && self.subscriptions_accept_any_topic_at_position[i] > 0u32 {
                        self.subscriptions_accept_any_topic_at_position[i] -= Nat::from(1u32);
                    }
                } 
                else {
                    for topic in topics_at_pos {
                        if let Some(count) = self.topic_counts[i].get_mut(topic) {
                            if *count > 0u32 {
                                *count -= Nat::from(1u32);
                                if *count == 0u32 {
                                    self.topics[i].remove(topic);
                                    self.topic_counts[i].remove(topic);
                                }
                            }
                        }
                    }
                }
            }
        }
        else {
            // No topics => decrement "accept any" count for all positions
            for i in 0..MAX_TOPICS {
                if self.subscriptions_accept_any_topic_at_position[i] > 0u32 { // always should happen
                    self.subscriptions_accept_any_topic_at_position[i] -= Nat::from(1u32);
                }
            }
        }
        ic_cdk::println!("Bit Mask after filter_remove: {:?}", self.subscriptions_accept_any_topic_at_position);
        ic_cdk::println!("Topics after filter_remove: {:?}", self.topics);
    }

    /// Computes a combined list of topics.
    ///
    /// # Explanation
    /// - If any position has a subscription that accepts any topic, we cannot strictly
    ///   combine all topics into a deterministic set for that position (since it's unconstrained).
    /// - If all positions have at least one specific topic set and no "accept any" subscriptions,
    ///   we return the combined specific topics for each position.
    /// - Otherwise, we return whatever positions we can combine.
    pub fn get_combined_topics(&self) -> Option<Vec<TopicsPosition>> {
        let mut combined_topics = Vec::new();
        let mut include_positions = true;

        for i in 0..self.topics.len() {
            if self.subscriptions_accept_any_topic_at_position.len() > i &&
               self.subscriptions_accept_any_topic_at_position[i] > 0u32 {
                include_positions = false;
                break;
            }
            else if !self.topics[i].is_empty() {
                combined_topics.push(self.topics[i].iter().cloned().collect());
            }
            else {
                include_positions = false;
                break;
            }
        }

        if include_positions && !combined_topics.is_empty() {
            Some(combined_topics)
        } else if !combined_topics.is_empty() {
            Some(combined_topics)
        } else {
            None 
        }
    }

    /// Returns a tuple of all active addresses and the combined topics, if any.
    pub fn get_active_addresses_and_topics(&self) -> (Vec<String>, Option<Vec<TopicsPosition>>) {
        let addresses: Vec<String> = self.addresses.keys().cloned().collect();

        let topics = self.get_combined_topics();

        (addresses, topics)
    }

}
