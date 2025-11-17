use std::collections::HashSet;
use std::io;

use crate::manifest::Dependency;
use crate::ostree::fetch_requires_from_repo;

/// Resolve the full dependency closure using an OSTree repository
pub fn resolve_dependency_closure(
    dependencies: &[Dependency],
    repo_path: &str,
) -> io::Result<Vec<String>> {
    resolve_dependency_closure_with_fetch(dependencies, |commit| {
        fetch_requires_from_repo(repo_path, commit)
    })
}

/// Resolve the full dependency closure (transitive dependencies) with custom fetch
pub fn resolve_dependency_closure_with_fetch<F>(
    dependencies: &[Dependency],
    mut fetch: F,
) -> io::Result<Vec<String>>
where
    F: FnMut(&str) -> io::Result<Vec<String>>,
{
    let mut resolved = Vec::new();
    let mut seen = HashSet::new();
    let mut visiting = HashSet::new();
    let mut stack = Vec::new();

    for dep in dependencies {
        visit_commit(
            &dep.commit,
            &mut fetch,
            &mut seen,
            &mut visiting,
            &mut stack,
            &mut resolved,
        )?;
    }

    Ok(resolved)
}

/// Visit a commit in dependency graph traversal (DFS with cycle detection)
fn visit_commit<F>(
    commit: &str,
    fetch: &mut F,
    seen: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
    stack: &mut Vec<String>,
    resolved: &mut Vec<String>,
) -> io::Result<()>
where
    F: FnMut(&str) -> io::Result<Vec<String>>,
{
    if seen.contains(commit) {
        return Ok(());
    }

    if visiting.contains(commit) {
        let cycle_path = build_cycle_path(stack, commit);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Circular dependency detected: {}", cycle_path),
        ));
    }

    visiting.insert(commit.to_string());
    stack.push(commit.to_string());

    let transitive_requires = fetch(commit)?;

    for req in &transitive_requires {
        visit_commit(req, fetch, seen, visiting, stack, resolved)?;
    }

    stack.pop();
    visiting.remove(commit);
    seen.insert(commit.to_string());
    resolved.push(commit.to_string());

    Ok(())
}

/// Build a human-readable cycle path string
fn build_cycle_path(stack: &[String], repeat: &str) -> String {
    let mut path = stack.to_vec();
    path.push(repeat.to_string());

    path.join(" -> ")
}
