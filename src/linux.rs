use std::path::Path;
use nix::sys::signal::{SigNum};
use nix::sched as consts;

use {Command, Namespace};

impl Command {

    /// Allow child process to daemonize. By default we run equivalent of
    /// `set_parent_death_signal(SIGKILL)`. See the `set_parent_death_signal`
    /// for better explanation.
    pub fn allow_daemonize(&mut self) {
        self.config.death_sig = None;
    }

    /// Set a signal that is sent to a process when it's parent is dead.
    /// This is by default set to `SIGKILL`. And you should keep it that way
    /// unless you know what you are doing.
    ///
    /// Particularly you should consider the following choices:
    ///
    /// 1. Instead of setting ``PDEATHSIG`` to some other signal, send signal
    ///    yourself and wait until child gracefully finishes.
    ///
    /// 2. Instead of daemonizing use ``systemd``/``upstart``/whatever system
    ///    init script to run your service
    ///
    /// Another issue with this option is that it works only with immediate
    /// child. To better control all descendant processes you may need the
    /// following:
    ///
    /// 1. The `prctl(PR_SET_CHILD_SUBREAPER..)` in parent which allows to
    ///    "catch" descendant processes.
    ///
    /// 2. The pid namespaces
    ///
    /// The former is out of scope of this library. The latter works by
    /// ``cmd.unshare(Namespace::Pid)``, but you may need to setup mount points
    /// and other important things (which are out of scope too).
    ///
    /// To reset this behavior use ``allow_daemonize()``.
    ///
    pub fn set_parent_death_signal(&mut self, sig: SigNum) {
        self.config.death_sig = Some(sig);
    }

    /// Set chroot dir. Only absolute path is supported
    ///
    /// This method has a non-standard security feature: even if current_dir
    /// is unspecified we set it to the directory inside the new root dir.
    /// see more details in the description of `Command::current_dir`.
    ///
    /// Note that if both chroot dir and pivot_root specified. The chroot dir
    /// is applied after pivot root. If chroot dir is relative it's relative
    /// to either suffix of the current directory with stripped off pivot dir
    /// or the pivot dir itself (if old workdir is not prefixed by pivot dir)
    ///
    /// # Panics
    ///
    /// If directory is not absolute
    pub fn chroot_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command
    {
        let dir = dir.as_ref();
        if !dir.is_absolute() {
            panic!("Chroot dir must be absolute");
        }
        self.chroot_dir = Some(dir.to_path_buf());

        self
    }

    /// Moves the root of the file system to the directory `put_old` and
    /// makes `new_root` the new root file system. Also it's optionally
    /// unmount `new_root` mount point after moving root (but it must exist
    /// anyway).
    ///
    /// The documentation says that `put_old` must be underneath the
    /// `new_root`.  Currently we have a restriction that both must be absolute
    /// and `new_root` be prefix of `put_old`, but we may lift it later.
    ///
    /// **Warning** if you don't unshare the mount namespace you will get
    /// moved filesystem root for *all processes running in that namespace*
    /// including parent (currently running) process itself. If you don't
    /// run equivalent to ``mount --make-private`` for the old root filesystem
    /// and set ``unmount`` to true, you may get unmounted filesystem for
    /// running processes too.
    ///
    /// See `man 2 pivot` for further details
    ///
    /// Note that if both chroot dir and pivot_root specified. The chroot dir
    /// is applied after pivot root.
    ///
    /// # Panics
    ///
    /// Panics if either path is not absolute or new_root is not a prefix of
    /// put_old.
    pub fn pivot_root<A: AsRef<Path>, B:AsRef<Path>>(&mut self,
        new_root: A, put_old: B, unmount: bool)
        -> &mut Command
    {
        let new_root = new_root.as_ref();
        let put_old = put_old.as_ref();
        if !new_root.is_absolute() {
            panic!("New root must be absolute");
        };
        if !put_old.is_absolute() {
            panic!("The `put_old` dir must be absolute");
        }
        let mut old_cmp = put_old.components();
        for (n, o) in new_root.components().zip(old_cmp.by_ref()) {
            if n != o {
                panic!("The new_root is not a prefix of put old");
            }
        }
        self.pivot_root = Some((new_root.to_path_buf(), put_old.to_path_buf(),
                                unmount));
        self
    }

    /// Unshare given namespaces
    ///
    /// Note: each namespace have some consequences on how new process will
    /// work, some of them are described in the `Namespace` type documentation.
    pub fn unshare<I:IntoIterator<Item=Namespace>>(&mut self, iter: I) {
        use Namespace::*;
        for ns in iter {
            self.config.namespaces |= match ns {
                Mount => consts::CLONE_NEWNS,
                Uts => consts::CLONE_NEWUTS,
                Ipc => consts::CLONE_NEWIPC,
                User => consts::CLONE_NEWUSER,
                Pid => consts::CLONE_NEWPID,
                Net => consts::CLONE_NEWNET,
            };
        }
    }

    /// Enables delivering of `SIGCHLD`
    ///
    /// Note the following things:
    ///
    /// 1. Unlike in most other implementations it's disabled by default
    /// 2. Default disposition of `SIGCHLD` is `Ignore`, so you may need
    ///    `sigaction` or `signalfd` to get use of it even after enabling
    /// 3. You may get `SIGCHLD` anyway even if you never enable this option by
    ///    the following means:
    ///      * Processes run by other libraries
    ///      * Children reparented to this process (*)
    ///
    /// (*) You may get children reparented to your process because of:
    ///
    /// 1. Your process has PID 1 (root of pid namespace/container/system)
    /// 2. Your process has called `prctl(PR_SET_CHILD_SUBREAPER)`
    pub fn enable_child_signal(&mut self) {
        self.config.sigchld = true;
    }

}
