//! The generator's background rebuild hook: a character rebuild can run on
//! a worker and apply its result on the main thread, or run
//! inline when no runner is installed.
//!
//! The original keeps a function-pointer chain. Exarust keeps the same
//! shape in Rust closures; the running game installs no runner, so a
//! rebuild runs inline.

/// A deferred rebuild job.
type Job = Box<dyn FnOnce()>;
/// A worker that takes a job.
type Runner = Box<dyn FnMut(Job)>;

/// A rebuild's result and the work that applies it.
#[derive(Default)]
pub struct Background {
    runner: Option<Runner>,
}

impl Background {
    /// Installs the worker hook; a second install replaces
    /// the old hook.
    pub fn install(&mut self, runner: Runner) {
        self.runner = Some(runner);
    }

    /// Whether a worker is installed.
    pub fn has_runner(&self) -> bool {
        self.runner.is_some()
    }

    /// Runs a rebuild: build, then apply, either inline or through the
    /// installed runner. The build runs before the apply in both paths.
    pub fn dispatch<R: 'static>(
        &mut self,
        build: impl FnOnce() -> R + 'static,
        apply: impl FnOnce(R) + 'static,
    ) {
        match self.runner.as_mut() {
            Some(runner) => runner(Box::new(move || apply(build()))),
            None => apply(build()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn without_a_runner_a_rebuild_is_inline() {
        let order = Rc::new(RefCell::new(Vec::new()));
        let a = order.clone();
        let b = order.clone();
        Background::default().dispatch(
            move || {
                a.borrow_mut().push("build");
                7
            },
            move |v| b.borrow_mut().push(if v == 7 { "apply" } else { "bad" }),
        );
        assert_eq!(*order.borrow(), ["build", "apply"]);
    }

    #[test]
    fn a_runner_can_defer_the_work() {
        let mut bg = Background::default();
        let queue: Rc<RefCell<Vec<Job>>> = Rc::new(RefCell::new(Vec::new()));
        let q = queue.clone();
        bg.install(Box::new(move |job| q.borrow_mut().push(job)));
        assert!(bg.has_runner());
        let got = Rc::new(RefCell::new(0));
        let g = got.clone();
        bg.dispatch(move || 3, move |v| *g.borrow_mut() = v);
        assert_eq!(*got.borrow(), 0);
        let jobs: Vec<_> = queue.borrow_mut().drain(..).collect();
        for job in jobs {
            job();
        }
        assert_eq!(*got.borrow(), 3);
    }
}
