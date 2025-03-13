use core::fmt::Debug;

use alloc::{
    string::{String, ToString},
    sync::{Arc, Weak},
    vec::Vec,
};
use sync::{LazyInit, Mutex};
use vfscore::{INodeInterface, OpenFlags, VfsError};

pub struct DentryNode {
    pub filename: String,
    pub node: Arc<dyn INodeInterface>,
    pub parent: Weak<DentryNode>,
    pub children: Mutex<Vec<Arc<DentryNode>>>,
}

impl Debug for DentryNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DentryNode")
            .field("filename", &self.filename)
            .field("parent", &self.parent)
            .field("children", &self.children)
            .finish()
    }
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
    pub fn mount(path: String, node: Arc<dyn INodeInterface>) -> Result<(), VfsError> {
        let paths = path.split("/").into_iter();
        let mut dentry = DENTRY_TREE.lock().clone();

        for x in paths {
            dentry = match x {
                "." | "" => dentry,
                ".." => dentry.parent.upgrade().unwrap_or(dentry),
                filename => {
                    let finded = dentry
                        .children
                        .lock()
                        .iter()
                        .find(|x| x.filename == *filename)
                        .cloned();
                    log::debug!("open: {}", filename);
                    match finded {
                        Some(new_dentry) => new_dentry,
                        None => dentry.node.open(filename, OpenFlags::NONE).map(|x| {
                            Arc::new(DentryNode::new(
                                filename.to_string(),
                                x,
                                Arc::downgrade(&dentry),
                            ))
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
        Ok(())
    }

    pub fn unmount(_path: String) -> Result<(), VfsError> {
        todo!("unmount in dentry node");
    }

    pub fn open(self: Arc<DentryNode>, name: &str, flags: OpenFlags) -> Option<Arc<DentryNode>> {
        let mut children = self.children.lock();
        if let Some(dnode) = children.iter().find(|x| x.filename == name) {
            Some(dnode.clone())
        } else {
            match self.node.open(name, flags) {
                Ok(node) => {
                    // add node to dentry node children.
                    let child_dentry = Arc::new(DentryNode::new(
                        name.to_string(),
                        node,
                        Arc::downgrade(&self),
                    ));
                    children.push(child_dentry.clone());
                    Some(child_dentry)
                }
                Err(_) => None,
            }
        }
    }

    pub fn path(&self) -> String {
        if let Some(_) = self.parent.upgrade() {
            let mut path = String::from("/") + &self.filename.clone();
            let mut pd = self.parent.clone();
            while let Some(parent) = pd.upgrade()
                && parent.filename != "/"
            {
                path = String::from("/") + &parent.filename + &path;
                pd = parent.parent.clone();
            }
            path
        } else {
            String::from("/")
        }
    }
}

pub static DENTRY_TREE: LazyInit<Mutex<Arc<DentryNode>>> = LazyInit::new();

/// dentry_open function will open the dentry node by path and dentry.
/// path should will be rebuild.
/// flags not be used at now.
pub fn dentry_open(
    mut dentry: Arc<DentryNode>,
    path: &str,
    flags: OpenFlags,
) -> Result<Arc<DentryNode>, VfsError> {
    if path.starts_with("/") {
        dentry = DENTRY_TREE.lock().clone();
    }
    let mut path_peeker = path.split("/").peekable();
    while let Some(filename) = path_peeker.next() {
        let new_dentry = match filename {
            "." | "" => Some(dentry.clone()),
            ".." => Some(dentry.parent.upgrade().unwrap_or(dentry.clone())),
            x => dentry.clone().open(x, flags.clone()),
        };
        if let Some(new_dentry) = new_dentry {
            dentry = new_dentry;
        } else if flags.contains(OpenFlags::O_CREAT) {
            // is not the last item
            let node = if path_peeker.peek().is_some() || flags.contains(OpenFlags::O_DIRECTORY) {
                dentry.node.mkdir(filename)?
            } else {
                dentry.node.touch(filename)?
            };
            let new_dentry = Arc::new(DentryNode::new(
                filename.to_string(),
                node,
                Arc::downgrade(&dentry),
            ));
            dentry.children.lock().push(new_dentry.clone());
            dentry = new_dentry;
        } else {
            return Err(VfsError::FileNotFound);
        }
    }
    Ok(dentry)
}

/// dentry_init will initialize the dentry tree.
/// but root node can't be modified more than once.
pub fn dentry_init(root_node: Arc<dyn INodeInterface>) {
    log::info!("Initialize dentry tree by root node");
    DENTRY_TREE.init_by(Mutex::new(Arc::new(DentryNode::new(
        String::from("/"),
        root_node,
        Weak::new(),
    ))))
}

pub fn dentry_root() -> Arc<DentryNode> {
    DENTRY_TREE.lock().clone()
}
