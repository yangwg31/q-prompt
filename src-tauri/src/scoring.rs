use crate::store::PromptItem;

pub fn calculate_score(item: &PromptItem, now: u64) -> f64 {
    let use_count = item.use_count as f64;
    let elapsed_secs = now.saturating_sub(item.last_used) as f64;
    let days_since = elapsed_secs / 86400.0;
    let recency = (-days_since / 30.0).exp();
    use_count * 0.6 + recency * 0.4
}

pub fn top_k(items: &[PromptItem], k: usize, now: u64) -> Vec<PromptItem> {
    let mut scored: Vec<(f64, &PromptItem)> = items
        .iter()
        .map(|item| (calculate_score(item, now), item))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(k)
        .map(|(_, item)| item.clone())
        .collect()
}
