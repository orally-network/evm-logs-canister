use std::collections::{HashMap, HashSet};
use candid::Nat;
use evm_logs_types::Filter;

type TopicsPosition = Vec<String>;

pub struct FilterManager {
    topics: Vec<HashSet<String>>,               
    topic_counts: Vec<HashMap<String, Nat>>,
    addresses: HashMap<String, Nat>,    
    pub subscriptions_accept_any_topic_at_position: Vec<Nat>, 
    total_subscriptions: Nat,                   
}

impl FilterManager {
    pub fn new() -> Self {
        FilterManager {
            topics: Vec::new(),
            topic_counts: Vec::new(),
            addresses: HashMap::new(),
            subscriptions_accept_any_topic_at_position: Vec::new(),
            total_subscriptions: Nat::from(0u32),
        }
    }

    pub fn add_filter(&mut self, filter: &Filter) {
        self.total_subscriptions += Nat::from(1u32);
    
        for address in &filter.addresses {
            *self.addresses.entry(address.clone()).or_insert(Nat::from(0u32)) += Nat::from(1u32);
        }

        if let Some(filter_topics) = &filter.topics {
            let num_positions = filter_topics.len();
            for i in 0..4 {
                while self.topics.len() <= i {
                    self.topics.push(HashSet::new());
                    self.topic_counts.push(HashMap::new());
                    self.subscriptions_accept_any_topic_at_position.push(Nat::from(0u32));
                }
    
                if i < num_positions {
                    let topics_at_pos = &filter_topics[i];
                    if topics_at_pos.is_empty() {
                        self.subscriptions_accept_any_topic_at_position[i] += Nat::from(1u32);
                    } else {
                        for topic in topics_at_pos {
                            self.topics[i].insert(topic.clone());
                            *self.topic_counts[i].entry(topic.clone()).or_insert(Nat::from(0u32)) += Nat::from(1u32);
                        }
                    }
                } else {
                    self.subscriptions_accept_any_topic_at_position[i] += Nat::from(1u32);
                }
            }
        } else {
            for i in 0..4 {
                while self.subscriptions_accept_any_topic_at_position.len() <= i {
                    self.subscriptions_accept_any_topic_at_position.push(Nat::from(0u32));
                }
                self.subscriptions_accept_any_topic_at_position[i] += Nat::from(1u32);
            }
        }
    }
    

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
        } else {
            for i in 0..self.subscriptions_accept_any_topic_at_position.len() {
                if self.subscriptions_accept_any_topic_at_position[i] > 0u32 {
                    self.subscriptions_accept_any_topic_at_position[i] -= Nat::from(1u32);
                }
            }
        }
    }

    pub fn get_combined_topics(&self) -> Option<Vec<TopicsPosition>> {
        let mut combined_topics = Vec::new();
        let mut include_positions = true;

        for i in 0..self.topics.len() {
            if self.subscriptions_accept_any_topic_at_position.len() > i &&
               self.subscriptions_accept_any_topic_at_position[i] > 0u32 {
                include_positions = false;
                break;
            } else if !self.topics[i].is_empty() {
                combined_topics.push(self.topics[i].iter().cloned().collect());
            } else {
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

    pub fn get_active_addresses_and_topics(&self) -> (Vec<String>, Option<Vec<Vec<String>>>) {
        let addresses: Vec<String> = self.addresses.keys().cloned().collect();

        let topics = self.get_combined_topics();

        (addresses, topics)
    }

}
