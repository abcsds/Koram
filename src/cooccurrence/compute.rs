use crate::cooccurrence::{CoOccurrenceResult, PairCount, PersonNode};
use std::collections::{HashMap, HashSet};

/// Pure core: given the per-person sweep results, build the co-occurrence result.
///
/// `selected` — the set of person IDs the user picked.
/// `assets` — for each asset ID, the set of person IDs Immich attributes to it (across the union of all sweeps).
/// `totals` — for each person in `selected`, the count of assets they appeared in.
/// `people_meta` — id → optional name, for the selected people.
pub fn build_result(
    selected: &HashSet<String>,
    assets: &HashMap<String, HashSet<String>>,
    totals: &HashMap<String, u32>,
    people_meta: &HashMap<String, Option<String>>,
    computed_at: String,
    from: Option<String>,
    to: Option<String>,
) -> CoOccurrenceResult {
    let mut pairs: HashMap<(String, String), u32> = HashMap::new();
    for (_asset_id, people_in_asset) in assets {
        let intersection: Vec<&String> = people_in_asset.iter().filter(|p| selected.contains(*p)).collect();
        for i in 0..intersection.len() {
            for j in (i + 1)..intersection.len() {
                let (a, b) = if intersection[i] < intersection[j] {
                    (intersection[i].clone(), intersection[j].clone())
                } else {
                    (intersection[j].clone(), intersection[i].clone())
                };
                *pairs.entry((a, b)).or_insert(0) += 1;
            }
        }
    }

    let mut people: Vec<PersonNode> = selected.iter().map(|id| PersonNode {
        id: id.clone(),
        name: people_meta.get(id).cloned().flatten(),
        total: *totals.get(id).unwrap_or(&0),
    }).collect();
    people.sort_by(|a, b| a.id.cmp(&b.id));

    let mut pair_vec: Vec<PairCount> = pairs.into_iter()
        .map(|((a, b), count)| PairCount { a, b, count })
        .collect();
    pair_vec.sort_by(|x, y| y.count.cmp(&x.count).then_with(|| x.a.cmp(&y.a)));

    CoOccurrenceResult { people, pairs: pair_vec, computed_at, from, to }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    fn s(v: &[&str]) -> HashSet<String> { v.iter().map(|x| x.to_string()).collect() }
    fn m(v: &[(&str, &[&str])]) -> HashMap<String, HashSet<String>> {
        v.iter().map(|(k, vs)| (k.to_string(), s(vs))).collect()
    }
    fn t(v: &[(&str, u32)]) -> HashMap<String, u32> {
        v.iter().map(|(k, c)| (k.to_string(), *c)).collect()
    }
    fn names(v: &[(&str, Option<&str>)]) -> HashMap<String, Option<String>> {
        v.iter().map(|(k, n)| (k.to_string(), n.map(String::from))).collect()
    }

    #[test]
    fn empty_selection_empty_result() {
        let r = build_result(&s(&[]), &m(&[]), &t(&[]), &names(&[]), "now".into(), None, None);
        assert_eq!(r.people.len(), 0);
        assert_eq!(r.pairs.len(), 0);
    }

    #[test]
    fn single_person_no_pairs() {
        let r = build_result(
            &s(&["A"]),
            &m(&[("img1", &["A"]), ("img2", &["A"])]),
            &t(&[("A", 2)]),
            &names(&[("A", Some("Alice"))]),
            "now".into(), None, None,
        );
        assert_eq!(r.people.len(), 1);
        assert_eq!(r.people[0].total, 2);
        assert_eq!(r.pairs.len(), 0);
    }

    #[test]
    fn three_people_overlapping() {
        // imgs:
        //   1: A,B
        //   2: A,B,C
        //   3: A,C
        //   4: B,C
        // selected: {A,B,C}
        // expected pairs: (A,B):2, (A,C):2, (B,C):2
        let r = build_result(
            &s(&["A", "B", "C"]),
            &m(&[
                ("1", &["A", "B"]),
                ("2", &["A", "B", "C"]),
                ("3", &["A", "C"]),
                ("4", &["B", "C"]),
            ]),
            &t(&[("A", 3), ("B", 3), ("C", 3)]),
            &names(&[("A", None), ("B", None), ("C", None)]),
            "now".into(), None, None,
        );
        assert_eq!(r.pairs.len(), 3);
        let lookup: HashMap<(String, String), u32> =
            r.pairs.iter().map(|p| ((p.a.clone(), p.b.clone()), p.count)).collect();
        assert_eq!(lookup[&("A".into(), "B".into())], 2);
        assert_eq!(lookup[&("A".into(), "C".into())], 2);
        assert_eq!(lookup[&("B".into(), "C".into())], 2);
    }

    #[test]
    fn unselected_people_dont_create_edges() {
        // C is in the asset but not selected → no edges involving C
        let r = build_result(
            &s(&["A", "B"]),
            &m(&[("1", &["A", "B", "C"])]),
            &t(&[("A", 1), ("B", 1)]),
            &names(&[("A", None), ("B", None)]),
            "now".into(), None, None,
        );
        assert_eq!(r.pairs.len(), 1);
        assert_eq!(r.pairs[0].a, "A");
        assert_eq!(r.pairs[0].b, "B");
        assert_eq!(r.pairs[0].count, 1);
    }

    #[test]
    fn pair_key_is_sorted() {
        // Even if Immich returns Z before A in the people list, the pair key should be (A, Z)
        let r = build_result(
            &s(&["Z", "A"]),
            &m(&[("1", &["Z", "A"])]),
            &t(&[("A", 1), ("Z", 1)]),
            &names(&[("A", None), ("Z", None)]),
            "now".into(), None, None,
        );
        assert_eq!(r.pairs[0].a, "A");
        assert_eq!(r.pairs[0].b, "Z");
    }
}

use crate::error::Result;
use crate::immich_api::{ImmichClient, Person};
use crate::job::Progress;
use futures_util::stream::{self, StreamExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

const SWEEP_CONCURRENCY: usize = 8;

pub struct ComputeArgs {
    pub client: ImmichClient,
    pub selected_person_ids: Vec<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub people_meta: Vec<Person>, // names of all known people for label lookups
    pub cancel: CancellationToken,
    pub progress_tx: tokio::sync::broadcast::Sender<Progress>,
}

pub async fn compute(args: ComputeArgs) -> Result<CoOccurrenceResult> {
    use std::collections::{HashMap, HashSet};

    let total = args.selected_person_ids.len() as u32;
    let processed = Arc::new(RwLock::new(0u32));

    let assets: Arc<RwLock<HashMap<String, HashSet<String>>>> = Arc::default();
    let totals: Arc<RwLock<HashMap<String, u32>>> = Arc::default();

    let names_for_progress: HashMap<String, Option<String>> =
        args.people_meta.iter().map(|p| (p.id.clone(), p.name.clone())).collect();

    // Drain the per-person sweep with a strict in-flight cap of 8.
    let results: Vec<Result<()>> = stream::iter(args.selected_person_ids.iter().cloned())
        .map(|id| {
            let client = args.client.clone();
            let from = args.from.clone();
            let to = args.to.clone();
            let assets = assets.clone();
            let totals = totals.clone();
            let processed = processed.clone();
            let cancel = args.cancel.clone();
            let progress_tx = args.progress_tx.clone();
            let names = names_for_progress.clone();
            async move {
                if cancel.is_cancelled() { return Err(crate::error::Error::Cancelled); }
                let fetched = client.search_person_assets(&id, from.as_deref(), to.as_deref()).await?;
                if cancel.is_cancelled() { return Err(crate::error::Error::Cancelled); }

                {
                    let mut a = assets.write().await;
                    let mut t = totals.write().await;
                    t.insert(id.clone(), fetched.len() as u32);
                    for asset in &fetched {
                        let entry = a.entry(asset.id.clone()).or_default();
                        if let Some(people) = &asset.people {
                            for p in people { entry.insert(p.id.clone()); }
                        }
                        entry.insert(id.clone());
                    }
                }

                let mut p = processed.write().await;
                *p += 1;
                let _ = progress_tx.send(Progress {
                    status: "running".into(),
                    processed: *p,
                    total,
                    current_person_id: Some(id.clone()),
                    current_person_name: names.get(&id).cloned().flatten(),
                    message: None,
                });
                Ok(())
            }
        })
        .buffer_unordered(SWEEP_CONCURRENCY)
        .collect()
        .await;

    if args.cancel.is_cancelled() {
        return Err(crate::error::Error::Cancelled);
    }
    // Surface the first error. Sibling sweeps in flight at the same time may still be
    // running; their per-iteration cancel checks let them bail at the next safe point.
    if let Some(first_err) = results.into_iter().find_map(|r| r.err()) {
        args.cancel.cancel(); // signal the rest to abort early
        return Err(first_err);
    }

    let selected: HashSet<String> = args.selected_person_ids.iter().cloned().collect();
    let people_meta: HashMap<String, Option<String>> =
        args.people_meta.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
    let assets_owned = assets.read().await.clone();
    let totals_owned = totals.read().await.clone();

    Ok(build_result(
        &selected, &assets_owned, &totals_owned, &people_meta,
        chrono::Utc::now().to_rfc3339(),
        args.from, args.to,
    ))
}
