use i_slint_backend_testing::{ElementHandle, ElementRoot};

pub trait ElementHandleEx {
    fn find_by_accessible_id(component: &impl ElementRoot, id: &str) -> impl Iterator<Item = Self>;
}

impl ElementHandleEx for ElementHandle {
    fn find_by_accessible_id(component: &impl ElementRoot, id: &str) -> impl Iterator<Item = Self> {
        let id = id.to_string();
        let results = component
            .root_element()
            .query_descendants()
            .match_predicate(move |elem| elem.accessible_id().is_some_and(|candidate_id| candidate_id == id))
            .find_all();
        results.into_iter()
    }
}
