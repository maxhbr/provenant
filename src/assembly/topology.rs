use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::models::{DatasourceId, FileInfo, Package, TopLevelDependency};

use super::AssemblerConfig;
use super::npm_workspace_merge::{
    NpmWorkspaceDomain, NpmWorkspaceRootHint, apply_npm_workspace_domain,
    collect_npm_workspace_hints, plan_npm_workspace_domains,
};

pub(super) enum TopologyHint {
    NpmWorkspaceRoot(NpmWorkspaceRootHint),
}

pub(super) enum TopologyDomain {
    NpmWorkspace(NpmWorkspaceDomain),
}

pub(super) struct TopologyPlan {
    domains: Vec<TopologyDomain>,
    claimed_npm_dirs: HashSet<PathBuf>,
}

impl TopologyPlan {
    pub(super) fn build(files: &[FileInfo], dir_files: &HashMap<PathBuf, Vec<usize>>) -> Self {
        let hints: Vec<_> = collect_npm_workspace_hints(files)
            .into_iter()
            .map(TopologyHint::NpmWorkspaceRoot)
            .collect();

        let mut domains = Vec::new();
        let mut claimed_npm_dirs = HashSet::new();

        let npm_workspace_hints: Vec<_> = hints
            .iter()
            .filter_map(|hint| match hint {
                TopologyHint::NpmWorkspaceRoot(hint) => Some(hint),
            })
            .collect();

        for domain in plan_npm_workspace_domains(files, dir_files, &npm_workspace_hints) {
            claimed_npm_dirs.insert(domain.root_dir.clone());
            claimed_npm_dirs.extend(domain.members.iter().map(|member| member.dir_path.clone()));
            domains.push(TopologyDomain::NpmWorkspace(domain));
        }

        Self {
            domains,
            claimed_npm_dirs,
        }
    }

    pub(super) fn claims_directory_assembly(
        &self,
        config: &AssemblerConfig,
        file_indices: &[usize],
        files: &[FileInfo],
    ) -> bool {
        if !config
            .datasource_ids
            .contains(&DatasourceId::NpmPackageJson)
        {
            return false;
        }

        let Some(&first_idx) = file_indices.first() else {
            return false;
        };
        let Some(parent_dir) = Path::new(&files[first_idx].path).parent() else {
            return false;
        };

        self.claimed_npm_dirs.contains(parent_dir)
    }

    pub(super) fn apply_npm_workspace_domains(
        &self,
        files: &mut [FileInfo],
        packages: &mut Vec<Package>,
        dependencies: &mut Vec<TopLevelDependency>,
    ) {
        for domain in &self.domains {
            match domain {
                TopologyDomain::NpmWorkspace(domain) => {
                    apply_npm_workspace_domain(domain, files, packages, dependencies);
                }
            }
        }
    }
}
