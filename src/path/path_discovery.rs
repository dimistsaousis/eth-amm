use std::collections::{HashMap, HashSet};

use ethers::types::H160;
use itertools::Itertools;

pub fn get_all_token_paths<'a>(
    start_token: &'a H160,
    tokens_map: &'a HashMap<&H160, Vec<&'a H160>>,
    min_length: usize,
    max_length: usize,
) -> Vec<Vec<&'a H160>> {
    let mut paths = Vec::new();
    let mut visited = HashSet::new();
    let mut current_path = vec![start_token];
    let mut unique_paths = HashSet::new();
    find_paths_recursive(
        start_token,
        start_token,
        &tokens_map,
        min_length,
        max_length,
        &mut visited,
        &mut current_path,
        &mut paths,
        &mut unique_paths,
    );
    paths
}

fn find_paths_recursive<'b>(
    start_token: &'b H160,
    current_token: &'b H160,
    tokens_map: &'b HashMap<&H160, Vec<&'b H160>>,
    min_length: usize,
    max_length: usize,
    visited: &mut HashSet<&'b H160>,
    current_path: &mut Vec<&'b H160>,
    paths: &mut Vec<Vec<&'b H160>>,
    unique_paths: &mut HashSet<String>,
) {
    if current_path.len() > max_length {
        return;
    }

    if start_token == current_token && current_path.len() > 1 && current_path.len() >= min_length {
        let path_id = current_path
            .iter()
            .map(ToString::to_string)
            .sorted()
            .join(",");
        if unique_paths.insert(path_id) {
            paths.push(current_path.clone());
        }
        return;
    }

    if let Some(next_tokens) = tokens_map.get(current_token) {
        for next_token in next_tokens {
            if !visited.contains(next_token) {
                visited.insert(next_token);
                current_path.push(next_token);
                find_paths_recursive(
                    start_token,
                    next_token,
                    tokens_map,
                    min_length,
                    max_length,
                    visited,
                    current_path,
                    paths,
                    unique_paths,
                );
                visited.remove(next_token);
                current_path.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_book::AddressBook;

    fn setup(book: &AddressBook) -> HashMap<&H160, Vec<&H160>> {
        let tokens = vec![
            &book.mainnet.erc20["weth"],
            &book.mainnet.erc20["link"],
            &book.mainnet.erc20["matic"],
            &book.mainnet.erc20["usdt"],
        ];
        let mut tokens_map: HashMap<&H160, Vec<&H160>> = HashMap::new();
        for t1 in &tokens {
            let entry = tokens_map.entry(t1).or_insert_with(Vec::new);
            for t2 in &tokens {
                if t1 != t2 {
                    entry.push(t2);
                }
            }
        }
        tokens_map
    }

    fn assert_path_size(paths: Vec<Vec<&H160>>, min_size: usize, max_size: usize) {
        assert_eq!(paths.iter().map(|p| p.len()).min().unwrap(), min_size);
        assert_eq!(paths.iter().map(|p| p.len()).max().unwrap(), max_size);
    }

    #[tokio::test]
    async fn test_get_all_token_paths() {
        let book = AddressBook::new();
        let tokens_map = setup(&book);
        let paths = get_all_token_paths(&book.mainnet.erc20["weth"], &tokens_map, 3, 5);
        assert_eq!(paths.len(), 7);
    }

    #[tokio::test]
    async fn test_get_all_token_paths_min_length() {
        let book = AddressBook::new();
        let tokens_map = setup(&book);
        let paths = get_all_token_paths(&book.mainnet.erc20["weth"], &tokens_map, 5, 5);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths.last().unwrap().len(), 5);
    }

    #[tokio::test]
    async fn test_get_all_token_paths_min_length_2() {
        let book = AddressBook::new();
        let tokens_map = setup(&book);
        let paths = get_all_token_paths(&book.mainnet.erc20["weth"], &tokens_map, 4, 5);
        assert_eq!(paths.len(), 4, "{:?}", paths);
        assert_path_size(paths, 4, 5);
    }

    #[tokio::test]
    async fn test_get_all_token_paths_max_length() {
        let book = AddressBook::new();
        let tokens_map = setup(&book);
        let paths = get_all_token_paths(&book.mainnet.erc20["weth"], &tokens_map, 0, 2);
        assert_eq!(paths.len(), 0, "{:?}", paths);
    }

    #[tokio::test]
    async fn test_get_all_token_paths_max_length_1() {
        let book = AddressBook::new();
        let tokens_map = setup(&book);
        let paths = get_all_token_paths(&book.mainnet.erc20["weth"], &tokens_map, 0, 3);
        assert_eq!(paths.len(), 3, "{:?}", paths);
        assert_path_size(paths, 3, 3);
    }

    #[tokio::test]
    async fn test_get_all_token_paths_max_length_2() {
        let book = AddressBook::new();
        let tokens_map = setup(&book);
        let paths = get_all_token_paths(&book.mainnet.erc20["weth"], &tokens_map, 0, 4);
        assert_eq!(paths.len(), 6, "{:?}", paths);
        assert_path_size(paths, 3, 4);
    }
}
