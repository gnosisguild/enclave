use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
const BITS_PER_LEVEL: u32 = 5;
const CHUNK_SIZE: usize = 1 << BITS_PER_LEVEL;
const CHUNK_MASK: u64 = (CHUNK_SIZE - 1) as u64;

// #[derive(Serialize, Deserialize, Debug)]
// pub struct SerializedHamt<K, V> {
//     nodes: Vec<SerializedNode<K, V>>,
//     roots: Vec<HamtRoot>,
// }
//
// #[derive(Serialize, Deserialize, Debug)]
// struct HamtRoot {
//     root_node_id: usize,
//     size: usize,
// }

// #[derive(Serialize, Deserialize, Debug)]
// enum SerializedNode<K, V> {
//     Empty,
//     Leaf { hash: u64, key: K, value: V },
//     Internal { bitmap: u32, children: Vec<usize> },
//     Collision { hash: u64, entries: Vec<(K, V)> },
// }

#[derive(Clone)]
enum Node<K, V> {
    Empty,
    Leaf {
        hash: u64,
        key: K,
        value: V,
    },
    Internal {
        bitmap: u32,
        children: Vec<Arc<Node<K, V>>>,
    },
    Collision {
        hash: u64,
        entries: Vec<(K, V)>,
    },
}

pub struct Hamt<K, V> {
    root: Arc<Node<K, V>>,
    size: usize,
}

impl<K, V> Hamt<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    pub fn new() -> Self {
        Hamt {
            root: Arc::new(Node::Empty),
            size: 0,
        }
    }

    fn hash_key(key: &K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn chunk(hash: u64, shift: u32) -> usize {
        ((hash >> shift) & CHUNK_MASK) as usize
    }

    fn index_from_bitmap(bitmap: u32, bit: u32) -> usize {
        (bitmap & (bit - 1)).count_ones() as usize
    }

    pub fn insert(&self, key: K, value: V) -> Self {
        let hash = Self::hash_key(&key);
        let new_root = Self::insert_rec(self.root.clone(), hash, key, value, 0);
        Hamt {
            root: new_root,
            size: self.size + 1,
        }
    }

    fn insert_rec(
        node: Arc<Node<K, V>>,
        hash: u64,
        key: K,
        value: V,
        shift: u32,
    ) -> Arc<Node<K, V>> {
        // Arc here
        match node.as_ref() {
            Node::Empty => Arc::new(Node::Leaf { hash, key, value }),

            Node::Leaf {
                hash: existing_hash,
                key: existing_key,
                value: existing_value,
            } => {
                if *existing_hash == hash {
                    if existing_key == &key {
                        Arc::new(Node::Leaf { hash, key, value })
                    } else {
                        Arc::new(Node::Collision {
                            // Arc here
                            hash,
                            entries: vec![
                                (existing_key.clone(), existing_value.clone()),
                                (key, value),
                            ],
                        })
                    }
                } else {
                    let mut new_node = Arc::new(Node::Internal {
                        // Arc here
                        bitmap: 0,
                        children: Vec::new(),
                    });

                    new_node = Self::insert_rec(
                        new_node,
                        *existing_hash,
                        existing_key.clone(),
                        existing_value.clone(),
                        shift,
                    );

                    new_node = Self::insert_rec(new_node, hash, key, value, shift);
                    new_node
                }
            }

            Node::Internal { bitmap, children } => {
                let chunk = Self::chunk(hash, shift);
                let bit = 1u32 << chunk;
                let index = Self::index_from_bitmap(*bitmap, bit);

                if bitmap & bit == 0 {
                    let mut new_children = children.clone();
                    new_children.insert(index, Arc::new(Node::Leaf { hash, key, value }));

                    Arc::new(Node::Internal {
                        // Arc here
                        bitmap: bitmap | bit,
                        children: new_children,
                    })
                } else {
                    let child = children[index].clone();
                    let new_child =
                        Self::insert_rec(child, hash, key, value, shift + BITS_PER_LEVEL);

                    let mut new_children = children.clone();
                    new_children[index] = new_child;

                    Arc::new(Node::Internal {
                        // Arc here
                        bitmap: *bitmap,
                        children: new_children,
                    })
                }
            }

            Node::Collision {
                hash: collision_hash,
                entries,
            } => {
                if *collision_hash == hash {
                    let mut new_entries = entries.clone();

                    if let Some(pos) = new_entries.iter().position(|(k, _)| k == &key) {
                        new_entries[pos] = (key, value);
                    } else {
                        new_entries.push((key, value));
                    }

                    Arc::new(Node::Collision {
                        // Arc here
                        hash: *collision_hash,
                        entries: new_entries,
                    })
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let hash = Self::hash_key(key);
        Self::get_rec(&self.root, hash, key, 0)
    }

    fn get_rec<'a>(node: &'a Node<K, V>, hash: u64, key: &K, shift: u32) -> Option<&'a V> {
        match node {
            Node::Empty => None,

            Node::Leaf {
                hash: leaf_hash,
                key: leaf_key,
                value: leaf_value,
            } => {
                if *leaf_hash == hash && leaf_key == key {
                    Some(leaf_value)
                } else {
                    None
                }
            }

            Node::Internal { bitmap, children } => {
                let chunk = Self::chunk(hash, shift);
                let bit = 1u32 << chunk;

                if bitmap & bit == 0 {
                    None
                } else {
                    let index = Self::index_from_bitmap(*bitmap, bit);
                    Self::get_rec(&children[index], hash, key, shift + BITS_PER_LEVEL)
                }
            }

            Node::Collision {
                hash: collision_hash,
                entries,
            } => {
                if *collision_hash == hash {
                    entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
                } else {
                    None
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializedHamt<K, V> {
    nodes: Vec<SerializedNode<K, V>>,
    roots: Vec<HamtRoot>,
}

#[derive(Serialize, Deserialize, Debug)]
struct HamtRoot {
    root_node_id: usize,
    size: usize,
}

#[derive(Serialize, Deserialize, Debug)]
enum SerializedNode<K, V> {
    Empty,
    Leaf {
        hash: u64,
        key: K,
        value: V,
    },
    Internal {
        bitmap: u32,
        children: Vec<usize>, // Node IDs instead of Arc pointers
    },
    Collision {
        hash: u64,
        entries: Vec<(K, V)>,
    },
}

impl<K, V> Hamt<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + Serialize,
    V: Clone + Send + Sync + Serialize,
{
    pub fn serialize_multiple(hamts: &[&Hamt<K, V>]) -> SerializedHamt<K, V> {
        let mut node_map: HashMap<*const Node<K, V>, usize> = HashMap::new();
        let mut nodes: Vec<SerializedNode<K, V>> = Vec::new();
        let mut roots: Vec<HamtRoot> = Vec::new();

        for hamt in hamts {
            let root_id = Self::serialize_node(&hamt.root, &mut node_map, &mut nodes);
            roots.push(HamtRoot {
                root_node_id: root_id,
                size: hamt.size,
            });
        }

        SerializedHamt { nodes, roots }
    }

    fn serialize_node(
        node: &Arc<Node<K, V>>,
        node_map: &mut HashMap<*const Node<K, V>, usize>,
        nodes: &mut Vec<SerializedNode<K, V>>,
    ) -> usize {
        let node_ptr = Arc::as_ptr(node);

        // Check if we've already serialized this exact node
        if let Some(&id) = node_map.get(&node_ptr) {
            return id;
        }

        let serialized = match node.as_ref() {
            Node::Empty => SerializedNode::Empty,
            Node::Leaf { hash, key, value } => SerializedNode::Leaf {
                hash: *hash,
                key: key.clone(),
                value: value.clone(),
            },
            Node::Internal { bitmap, children } => {
                // Recursively serialize children FIRST, getting their IDs
                let child_ids: Vec<usize> = children
                    .iter()
                    .map(|child| Self::serialize_node(child, node_map, nodes))
                    .collect();
                SerializedNode::Internal {
                    bitmap: *bitmap,
                    children: child_ids,
                }
            }
            Node::Collision { hash, entries } => SerializedNode::Collision {
                hash: *hash,
                entries: entries.clone(),
            },
        };

        // Now add THIS node after its children
        let node_id = nodes.len();
        nodes.push(serialized);
        node_map.insert(node_ptr, node_id);
        node_id
    }

    pub fn deserialize_multiple(serialized: SerializedHamt<K, V>) -> Vec<Hamt<K, V>>
    where
        K: DeserializeOwned,
        V: DeserializeOwned,
    {
        let mut node_cache: Vec<Arc<Node<K, V>>> = Vec::new();

        // Deserialize all nodes - children come before parents, so this works!
        for serialized_node in serialized.nodes {
            let node = match serialized_node {
                SerializedNode::Empty => Arc::new(Node::Empty),
                SerializedNode::Leaf { hash, key, value } => {
                    Arc::new(Node::Leaf { hash, key, value })
                }
                SerializedNode::Internal { bitmap, children } => {
                    let child_nodes = children.iter().map(|&id| node_cache[id].clone()).collect();
                    Arc::new(Node::Internal {
                        bitmap,
                        children: child_nodes,
                    })
                }
                SerializedNode::Collision { hash, entries } => {
                    Arc::new(Node::Collision { hash, entries })
                }
            };
            node_cache.push(node);
        }

        // Create HAMTs from roots
        serialized
            .roots
            .into_iter()
            .map(|root| Hamt {
                root: node_cache[root.root_node_id].clone(),
                size: root.size,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut map = Hamt::new(); // single-threaded

        map = map.insert("hello", 42);
        map = map.insert("world", 100);
        map = map.insert("rust", 200);

        assert_eq!(Some(&42), map.get(&"hello"));
        assert_eq!(Some(&100), map.get(&"world"));
        assert_eq!(Some(&200), map.get(&"rust"));
        assert_eq!(None, map.get(&"nope"));

        // Persistence - old map still valid
        let map2 = map.insert("hello", 999);
        assert_eq!(Some(&42), map.get(&"hello"));
        assert_eq!(Some(&999), map2.get(&"hello"));
    }

    #[test]
    fn test_serialization_deduplication() {
        let map1 = Hamt::new();
        let map1 = map1.insert("hello".to_string(), 42);
        let map1 = map1.insert("world".to_string(), 100);

        let map2 = map1.insert("hello".to_string(), 999); // Shares structure with map1

        // Serialize both maps together
        let serialized = Hamt::serialize_multiple(&[&map1, &map2]);

        println!("Total nodes serialized: {}", serialized.nodes.len());
        println!("Number of HAMTs: {}", serialized.roots.len());

        // Deserialize back
        let deserialized = Hamt::deserialize_multiple(serialized);

        assert_eq!(Some(&42), deserialized[0].get(&"hello".to_string()));
        assert_eq!(Some(&100), deserialized[0].get(&"world".to_string()));
        assert_eq!(Some(&999), deserialized[1].get(&"hello".to_string()));
        assert_eq!(Some(&100), deserialized[1].get(&"world".to_string()));
    }
}
