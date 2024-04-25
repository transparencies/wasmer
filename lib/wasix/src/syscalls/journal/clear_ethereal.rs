use super::*;

impl<'a, 'c> JournalSyscallPlayer<'a, 'c> {
    pub(super) fn clear_ethereal(
        &mut self,
        mut differ_ethereal: Option<&mut Vec<JournalEntry<'a>>>,
    ) {
        tracing::trace!("Replay journal - ClearEthereal");
        self.spawn_threads.clear();
        self.stdout.clear();
        self.stderr.clear();
        self.stdout_fds.clear();
        self.stderr_fds.clear();
        self.stdout_fds.insert(1 as WasiFd);
        self.stderr_fds.insert(2 as WasiFd);
        differ_ethereal.iter_mut().for_each(|e| e.clear());
        self.staged_differ_memory.clear();
    }
}
