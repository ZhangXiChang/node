use eyre::Result;

use crate::node::{NewNodeInfo, Node};

pub struct System {
    pub node: Node,
}
impl System {
    pub fn new() -> Result<Self> {
        Ok(Self {
            node: Node::new("测试节点".to_string(), NewNodeInfo::default())?,
        })
    }
}
impl Drop for System {
    fn drop(&mut self) {
        self.node.close(0, "节点实例被释放".as_bytes().to_vec())
    }
}
