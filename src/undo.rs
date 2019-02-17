#[cfg_attr(test, derive(Clone, Debug))]
pub struct Undo<T> {
    actions: Vec<T>,
    actions_performed: usize,
}

impl<T> Undo<T>
where
    T: PartialEq,
{
    pub fn new() -> Self {
        Self {
            actions: vec![],
            actions_performed: 0,
        }
    }

    /// When an action is performed, record the action in a log so it can later be undone.
    pub fn record(&mut self, action: T) {
        assert!(self.actions_performed <= self.actions.len());
        if self.actions.len() <= self.actions_performed {
            self.actions.push(action);
        } else {
            if self.actions[self.actions_performed] != action {
                self.actions.truncate(self.actions_performed + 1);
            }

            self.actions[self.actions_performed] = action;
        }

        self.actions_performed += 1;
        assert!(self.actions_performed <= self.actions.len());
    }

    /// Get the most recent action from the log.
    pub fn undo(&mut self) -> Option<&T> {
        assert!(self.actions_performed <= self.actions.len());

        let mut result = None;

        if self.actions_performed > 0 {
            result = self.actions.get(self.actions_performed - 1);
            self.actions_performed -= 1;
        }

        assert!(self.actions_performed <= self.actions.len());
        result
    }

    /// Return the most recently undone action.
    pub fn redo(&mut self) -> Option<&T> {
        assert!(self.actions_performed <= self.actions.len());
        let result = self.actions.get(self.actions_performed);
        if result.is_some() {
            self.actions_performed += 1;
        }
        assert!(self.actions_performed <= self.actions.len());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl<A: Arbitrary + Clone> Arbitrary for Undo<A> {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let actions = Vec::arbitrary(g);
            let actions_performed = actions.len();
            Undo {
                actions,
                actions_performed,
            }
        }
    }

    #[test]
    fn empty_should_return_none() {
        let mut sut: Undo<i8> = Undo::new();

        assert_eq!(sut.undo(), None);
        assert_eq!(sut.redo(), None);
    }

    #[quickcheck]
    fn undo_should_return_most_recent_action(mut sut: Undo<u32>, x: u32) {
        let num_actions = sut.actions_performed;
        sut.record(x);

        assert_eq!(sut.actions_performed, num_actions + 1);
        assert_eq!(sut.undo(), Some(&x));
        assert_eq!(sut.actions_performed, num_actions);
    }

    #[quickcheck]
    fn redo_should_return_most_recently_undone_action(mut sut: Undo<u32>, x: u32) {
        sut.record(x);
        let num_actions = sut.actions_performed;
        sut.undo();

        assert_eq!(sut.redo(), Some(&x));
        assert_eq!(sut.actions_performed, num_actions);
    }
}
