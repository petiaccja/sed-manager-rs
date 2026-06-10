use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use slint::{Model as _, VecModel};

/// An key-indexed wrapper around [`VecModel`].
///
/// Internally, the model is stored a [`VecModel`], and can be accessed via
/// [`model`].
///
/// You should NEVER modify (`push` & `remove`) the internal model, or else
/// you'll mess up the associations between the key and model index.
///
/// [`model`]: AssociativeModel::model
#[derive(Clone, Default)]
pub struct AssociativeModel<K, V> {
    inner: Rc<VecModel<V>>,
    indices: HashMap<K, usize>,
}

impl<K: Eq + Hash, T: Clone + 'static> AssociativeModel<K, T> {
    pub fn new() -> Self {
        Self { inner: Rc::new(VecModel::default()), indices: HashMap::new() }
    }

    /// Returns the underlying [`VecModel`] for use in Slint bindings and model
    /// adapters. The returned `Rc` shares ownership with this struct, so
    /// changes made through [`AssociativeModel`] are immediately visible
    /// through it.
    pub fn inner(&self) -> Rc<VecModel<T>> {
        self.inner.clone()
    }

    pub fn len(&self) -> usize {
        self.indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.indices.contains_key(key)
    }

    /// Returns a clone of the element associated with `key`, or `None`.
    pub fn get<Q>(&self, key: &Q) -> Option<T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.indices.get(key).and_then(|&idx| self.inner.row_data(idx))
    }

    /// Inserts a key-value pair. If the key was already present the existing
    /// element is replaced and the old value is returned. Otherwise the value
    /// is appended at the end of the model and `None` is returned.
    pub fn insert(&mut self, key: K, value: T) -> Option<T> {
        if let Some(&idx) = self.indices.get(&key) {
            let old = self.inner.row_data(idx);
            self.inner.set_row_data(idx, value);
            old
        } else {
            let index = self.inner.row_count();
            self.inner.push(value);
            self.indices.insert(key, index);
            None
        }
    }

    /// Replaces the element associated with `key`. Returns `true` if the key
    /// existed, `false` otherwise.
    pub fn set<Q>(&self, key: &Q, value: T) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(&idx) = self.indices.get(key) {
            self.inner.set_row_data(idx, value);
            true
        } else {
            false
        }
    }

    /// Applies `f` to the current element associated with `key` and stores the
    /// result. Returns `true` if the key existed, `false` otherwise.
    pub fn update<Q, F>(&self, key: &Q, f: F) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        F: FnOnce(T) -> T,
    {
        if let Some(&idx) = self.indices.get(key)
            && let Some(value) = self.inner.row_data(idx)
        {
            self.inner.set_row_data(idx, f(value));
            true
        } else {
            false
        }
    }

    /// Removes the element associated with `key`. All indices in the map that
    /// are greater than the removed index are decremented to stay consistent
    /// with the underlying `VecModel`. Returns `true` if the key existed.
    pub fn remove<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(removed_idx) = self.indices.remove(key) {
            self.inner.remove(removed_idx);
            for idx in self.indices.values_mut() {
                if *idx > removed_idx {
                    *idx -= 1;
                }
            }
            true
        } else {
            false
        }
    }

    /// Retains only the entries for which `predicate` returns `true`, removing
    /// the rest and keeping the underlying `VecModel` consistent.
    pub fn retain<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&K) -> bool,
    {
        // Collect indices being removed while pruning the map in one pass.
        let mut removed_indices: Vec<usize> = Vec::new();
        self.indices.retain(|k, v| {
            if predicate(k) {
                true
            } else {
                removed_indices.push(*v);
                false
            }
        });

        // Remove from VecModel in descending order so earlier indices remain
        // valid for subsequent removals.
        removed_indices.sort_unstable_by(|a, b| b.cmp(a));
        for &idx in &removed_indices {
            self.inner.remove(idx);
        }

        // Adjust each surviving entry by the number of removed indices that
        // fell below it.
        for idx in self.indices.values_mut() {
            let shift = removed_indices.iter().filter(|&&r| r < *idx).count();
            *idx -= shift;
        }
    }
}

impl<K, V> core::fmt::Debug for AssociativeModel<K, V>
where
    K: core::fmt::Debug,
    V: Clone + core::fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entries = self
            .indices
            .iter()
            .filter_map(|(name, index)| self.inner.row_data(*index).map(|value| (name, value)));
        f.debug_map().entries(entries).finish()

        // f.debug_struct("AssociativeModel")
        //     .field("inner", &self.inner)
        //     .field("indices", &self.indices)
        //     .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // push / contains_key / len / get
    // -------------------------------------------------------------------------

    #[test]
    fn insert_single_element() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        assert_eq!(m.insert("a".to_string(), 1), None);
        assert_eq!(m.len(), 1);
        assert!(m.contains_key("a"));
        assert_eq!(m.get("a"), Some(1));
    }

    #[test]
    fn insert_replaces_existing_and_returns_old_value() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        assert_eq!(m.insert("a".to_string(), 99), Some(1));
        assert_eq!(m.get("a"), Some(99));
        // The VecModel must still have exactly one row.
        assert_eq!(m.inner().row_count(), 1);
    }

    #[test]
    fn get_missing_key_returns_none() {
        let m: AssociativeModel<String, i32> = AssociativeModel::new();
        assert_eq!(m.get("a"), None);
    }

    #[test]
    fn is_empty_initially() {
        let m: AssociativeModel<String, i32> = AssociativeModel::new();
        assert!(m.is_empty());
    }

    #[test]
    fn is_not_empty_after_insert() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        assert!(!m.is_empty());
    }

    // -------------------------------------------------------------------------
    // set / update
    // -------------------------------------------------------------------------

    #[test]
    fn set_updates_existing_key() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        assert!(m.set("a", 99));
        assert_eq!(m.get("a"), Some(99));
    }

    #[test]
    fn set_returns_false_for_missing_key() {
        let m: AssociativeModel<String, i32> = AssociativeModel::new();
        assert!(!m.set("a", 1));
    }

    #[test]
    fn update_modifies_via_closure() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 10);
        assert!(m.update("a", |v| v * 2));
        assert_eq!(m.get("a"), Some(20));
    }

    #[test]
    fn update_returns_false_for_missing_key() {
        let m: AssociativeModel<String, i32> = AssociativeModel::new();
        assert!(!m.update("a", |v| v));
    }

    // -------------------------------------------------------------------------
    // remove — index-shifting correctness
    // -------------------------------------------------------------------------

    #[test]
    fn remove_returns_false_for_missing_key() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        assert!(!m.remove("a"));
    }

    #[test]
    fn remove_only_element() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        assert!(m.remove("a"));
        assert!(m.is_empty());
        assert!(!m.contains_key("a"));
    }

    #[test]
    fn remove_first_shifts_remaining_indices() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        m.insert("c".to_string(), 3);

        m.remove("a");

        assert!(!m.contains_key("a"));
        assert_eq!(m.get("b"), Some(2));
        assert_eq!(m.get("c"), Some(3));
        assert_eq!(m.len(), 2);
        assert_eq!(m.inner().row_count(), 2);
    }

    #[test]
    fn remove_middle_shifts_later_indices() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        m.insert("c".to_string(), 3);

        m.remove("b");

        assert_eq!(m.get("a"), Some(1));
        assert!(!m.contains_key("b"));
        assert_eq!(m.get("c"), Some(3));
        assert_eq!(m.len(), 2);
        assert_eq!(m.inner().row_count(), 2);
    }

    #[test]
    fn remove_last_does_not_shift_earlier_indices() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        m.insert("c".to_string(), 3);

        m.remove("c");

        assert_eq!(m.get("a"), Some(1));
        assert_eq!(m.get("b"), Some(2));
        assert!(!m.contains_key("c"));
        assert_eq!(m.len(), 2);
        assert_eq!(m.inner().row_count(), 2);
    }

    // -------------------------------------------------------------------------
    // underlying model consistency
    // -------------------------------------------------------------------------

    #[test]
    fn model_rc_shared_with_callers() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        let vm = m.inner();

        m.insert("a".to_string(), 42);

        // The Rc returned before the push still sees the new element.
        assert_eq!(vm.row_count(), 1);
        assert_eq!(vm.row_data(0), Some(42));
    }

    // -------------------------------------------------------------------------
    // retain
    // -------------------------------------------------------------------------

    #[test]
    fn retain_multiple_removals_shifts_correctly() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);
        m.insert("c".to_string(), 3);
        m.insert("d".to_string(), 4);

        // Remove "a" (idx 0) and "c" (idx 2), keeping "b" (idx 1) and "d" (idx 3).
        m.retain(|k| k == "b" || k == "d");

        assert_eq!(m.get("b"), Some(2));
        assert_eq!(m.get("d"), Some(4));
        assert_eq!(m.len(), 2);
        assert_eq!(m.inner().row_count(), 2);
    }

    #[test]
    fn retain_all_removed_leaves_empty_model() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);

        m.retain(|_| false);

        assert!(m.is_empty());
        assert_eq!(m.inner().row_count(), 0);
    }

    #[test]
    fn retain_none_removed_leaves_model_intact() {
        let mut m: AssociativeModel<String, i32> = AssociativeModel::new();
        m.insert("a".to_string(), 1);
        m.insert("b".to_string(), 2);

        m.retain(|_| true);

        assert_eq!(m.len(), 2);
        assert_eq!(m.inner().row_count(), 2);
    }
}
