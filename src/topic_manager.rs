use std::collections::{HashMap, HashSet};
use candid::Nat;

pub struct TopicManager {
    topics: Vec<HashSet<String>>,               
    topic_counts: Vec<HashMap<String, Nat>>,    
    pub subscriptions_accept_any_topic_at_position: Vec<Nat>, 
    total_subscriptions: Nat,                   
}

impl TopicManager {
    pub fn new() -> Self {
        TopicManager {
            topics: Vec::new(),
            topic_counts: Vec::new(),
            subscriptions_accept_any_topic_at_position: Vec::new(),
            total_subscriptions: Nat::from(0u32),
        }
    }

    pub fn add_filter(&mut self, filter_topics: &Option<Vec<Vec<String>>>) {
        self.total_subscriptions += Nat::from(1u32);
    
        if let Some(filter_topics) = filter_topics {
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
    

    pub fn remove_filter(&mut self, filter_topics: &Option<Vec<Vec<String>>>) {
        if self.total_subscriptions > Nat::from(0u32) {
            self.total_subscriptions -= Nat::from(1u32);
        }

        if let Some(filter_topics) = filter_topics {
            for (i, topics_at_pos) in filter_topics.iter().enumerate() {
                if topics_at_pos.is_empty() {
                    if self.subscriptions_accept_any_topic_at_position.len() > i {
                        if self.subscriptions_accept_any_topic_at_position[i] > Nat::from(0u32) {
                            self.subscriptions_accept_any_topic_at_position[i] -= Nat::from(1u32);
                        }
                    }
                } else {
                    for topic in topics_at_pos {
                        if let Some(count) = self.topic_counts[i].get_mut(topic) {
                            if *count > Nat::from(0u32) {
                                *count -= Nat::from(1u32);
                                if *count == Nat::from(0u32) {
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
                if self.subscriptions_accept_any_topic_at_position[i] > Nat::from(0u32) {
                    self.subscriptions_accept_any_topic_at_position[i] -= Nat::from(1u32);
                }
            }
        }
    }

    pub fn get_combined_topics(&self) -> Option<Vec<Vec<String>>> {
        let mut combined_topics = Vec::new();
        let mut include_positions = true;

        for i in 0..self.topics.len() {
            if self.subscriptions_accept_any_topic_at_position.len() > i &&
               self.subscriptions_accept_any_topic_at_position[i] > Nat::from(0u32) {
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
}
