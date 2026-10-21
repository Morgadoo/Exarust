//! The generator's background look rebuild: the look rebuild can run its
//! build step on a worker and its apply step on the main thread, or both
//! inline when no runner is installed.
//!
//! The original hands a runner the build and the apply separately, so the
//! runner decides where each runs. Exarust keeps that shape in Rust
//! closures; the running game installs no runner, so a rebuild runs inline.

use std::any::Any;

/// A rebuild's build step and its result.
pub type Build = Box<dyn FnOnce() -> Box<dyn Any>>;
/// The step that applies a build's result.
pub type Apply = Box<dyn FnOnce(Box<dyn Any>)>;
/// A runner: it takes the two steps and runs the build, then hands its
/// result to the apply, where and when it chooses.
pub type Runner = Box<dyn FnMut(Build, Apply)>;

/// The installed runner, if any.
#[derive(Default)]
pub struct Background {
    runner: Option<Runner>,
}

impl Background {
    /// Installs a runner; a second install replaces the first.
    pub fn install(&mut self, runner: Runner) {
        self.runner = Some(runner);
    }

    /// Whether a runner is installed.
    pub fn has_runner(&self) -> bool {
        self.runner.is_some()
    }

    /// Runs a rebuild: without a runner the build and then the apply run
    /// inline; with one, both steps go to the runner separately.
    pub fn dispatch<R: 'static>(
        &mut self,
        build: impl FnOnce() -> R + 'static,
        apply: impl FnOnce(R) + 'static,
    ) {
        match self.runner.as_mut() {
            Some(runner) => runner(
                Box::new(move || Box::new(build()) as Box<dyn Any>),
                Box::new(move |result| {
                    if let Ok(result) = result.downcast::<R>() {
                        apply(*result);
                    }
                }),
            ),
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

    /// A runner gets the two steps apart: it can run the build now and the
    /// apply later.
    #[test]
    fn a_runner_takes_the_build_and_the_apply_apart() {
        let mut bg = Background::default();
        type Pending = Rc<RefCell<Vec<(Box<dyn Any>, Apply)>>>;
        let applies: Pending = Rc::new(RefCell::new(Vec::new()));
        let q = applies.clone();
        bg.install(Box::new(move |build, apply| {
            let result = build();
            q.borrow_mut().push((result, apply));
        }));
        assert!(bg.has_runner());
        let got = Rc::new(RefCell::new(0));
        let g = got.clone();
        bg.dispatch(move || 3, move |v| *g.borrow_mut() = v);
        assert_eq!(*got.borrow(), 0);
        let pending: Vec<_> = applies.borrow_mut().drain(..).collect();
        for (result, apply) in pending {
            apply(result);
        }
        assert_eq!(*got.borrow(), 3);
    }
}
