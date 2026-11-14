use super::Shuffle;

#[test]
fn a_seeded_shuffle_is_repeatable() {
    let mut first = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut second = first.clone();
    Shuffle::new(7).reorder(&mut first);
    Shuffle::new(7).reorder(&mut second);
    assert_eq!(first, second);
}

#[test]
fn different_seeds_give_different_orders_sometimes() {
    let orders: std::collections::BTreeSet<Vec<u32>> = (1..40)
        .map(|seed| {
            let mut items: Vec<u32> = (0..8).collect();
            Shuffle::new(seed).reorder(&mut items);
            items
        })
        .collect();
    assert!(orders.len() > 5, "only {} distinct orders", orders.len());
}

#[test]
fn reordering_keeps_every_item() {
    let mut items: Vec<u32> = (0..100).collect();
    Shuffle::new(99).reorder(&mut items);
    items.sort_unstable();
    assert_eq!(items, (0..100).collect::<Vec<_>>());
}

#[test]
fn below_stays_in_range() {
    let mut shuffle = Shuffle::new(3);
    for n in 1..20 {
        for _ in 0..50 {
            assert!(shuffle.below(n) < n);
        }
    }
    assert_eq!(shuffle.below(0), 0);
}
