use std::sync::Arc;
use parking_lot::{Condvar, Mutex};

#[derive(Clone, Copy)]
pub enum WorkerCmd {
    Stream(bool),
    Close,
}

pub struct CmdChan {
    cmd: Arc<Mutex<(usize, WorkerCmd)>>,  // (msg_num: usize, worker_cmd: WorkerCmd)
    condvar: Arc<Condvar>,
}
impl CmdChan {
    pub fn new() -> Self {
        Self {
            cmd: Arc::new(Mutex::new((0, WorkerCmd::Close))),
            condvar: Arc::new(Condvar::new()),
        }
    }
    pub fn new_recvr(&self) -> CmdRecvr {
        // If this command channel has already been used, posted `msg_num` is not 0.
        // The new receiver should be initialized with this value
        // since the first message it will need to react on will be `msg_num + 1`
        let (msg_num, _cmd_val) = &*self.cmd.lock();
        let last_posted_msg_num = *msg_num;

        CmdRecvr {
            cmd: self.cmd.clone(),
            condvar: self.condvar.clone(),
            viewed_msg_num: last_posted_msg_num,
        }
    }
    pub fn send(&self, cmd: WorkerCmd) {
        let mut mutex_guard = self.cmd.lock();
        let (msg_num, cmd_val) = &mut *mutex_guard;
        *cmd_val = cmd;
        *msg_num += 1;
        self.condvar.notify_all();
    }
}

pub struct CmdRecvr {
    cmd: Arc<Mutex<(usize, WorkerCmd)>>,
    condvar: Arc<Condvar>,
    viewed_msg_num: usize,
}
impl CmdRecvr {
    pub fn recv(&mut self) -> Result<WorkerCmd, String> {
        let mut mutex_guard = self.cmd.lock();

        // Check if a new message has already been posted. Wait if not yet posted:
        let (msg_num, _cmd_val) = &*mutex_guard;
        if *msg_num == self.viewed_msg_num {
            self.condvar.wait(&mut mutex_guard);
        } else if *msg_num == self.viewed_msg_num + 1 {
            // The new command has already been published. No need to wait
        } else {
            return Err(format!("Viewed msg count {} diverged from the published command number {}", self.viewed_msg_num, *msg_num))
        };

        // Read and return the new command:
        let (_msg_num, cmd_val) = &*mutex_guard;
        self.viewed_msg_num += 1;
        Ok(*cmd_val)
    }
}