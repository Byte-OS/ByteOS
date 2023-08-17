use alloc::{
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use sync::{LazyInit, Mutex};
use vfscore::{INodeInterface, OpenFlags};

pub struct DentryNode {
    pub filename: String,
    pub node: Arc<dyn INodeInterface>,
    pub parent: Weak<DentryNode>,
    pub children: Mutex<Vec<Arc<DentryNode>>>,
}

impl DentryNode {
    pub fn new(filename: String, node: Arc<dyn INodeInterface>, parent: Weak<DentryNode>) -> Self {
        Self {
            filename,
            node,
            parent,
            children: Mutex::new(Vec::new()),
        }
    }

    /// Mount a fs to DentryTree, return Some if successfully mounted.
    /// path: The mounted path.
    /// node: fs root directory node.
    pub fn mount(path: String, node: Arc<dyn INodeInterface>) -> Option<()> {
        let paths = path.split("/").into_iter();
        let mut dentry = DENTRY_TREE.lock().clone();

        for x in paths {
            dentry = match x {
                "." => dentry,
                ".." => dentry.parent.upgrade().unwrap_or(dentry),
                filename => {
                    let finded = dentry
                        .children
                        .lock()
                        .iter()
                        .find(|x| x.filename == *filename)
                        .cloned();
                    match finded {
                        Some(new_dentry) => new_dentry,
                        None => dentry
                            .node
                            .open(filename, OpenFlags::NONE)
                            .map_or(None, |x| {
                                Some(Arc::new(DentryNode::new(
                                    filename.to_string(),
                                    x,
                                    Arc::downgrade(&dentry),
                                )))
                            })?,
                    }
                }
            }
        }
        dentry
            .parent
            .upgrade()
            .unwrap()
            .children
            .lock()
            .push(Arc::new(DentryNode::new(
                dentry.filename.clone(),
                node,
                dentry.parent.clone(),
            )));
        Some(())
    }

    pub fn add_child(&mut self, filename: String, node: Arc<dyn INodeInterface>) {
        self.children
            .lock()
            .push(Arc::new(Self::new(filename, node, Weak::new())));
    }
}

pub static DENTRY_TREE: LazyInit<Mutex<Arc<DentryNode>>> = LazyInit::new();
