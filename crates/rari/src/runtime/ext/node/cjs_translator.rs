// Copyright 2018-2025 the Deno authors. All rights reserved. MIT license.

use std::borrow::Cow;
use std::sync::Arc;

use deno_ast::MediaType;
use deno_ast::ModuleExportsAndReExports;
use deno_ast::ModuleSpecifier;
use deno_error::JsErrorBox;
use deno_resolver::npm::DenoInNpmPackageChecker;
use deno_runtime::deno_fs;
use node_resolver::DenoIsBuiltInNodeModuleChecker;
use node_resolver::analyze::CjsAnalysis as ExtNodeCjsAnalysis;
use node_resolver::analyze::CjsAnalysisExports;
use node_resolver::analyze::EsmAnalysisMode;
use rustc_hash::FxHashMap;
use serde::Deserialize;
use serde::Serialize;
use sys_traits::impls::RealSys;

use super::resolvers::NpmPackageFolderResolverImpl;
use super::resolvers::Resolver;

pub type NodeCodeTranslator = node_resolver::analyze::NodeCodeTranslator<
    CjsCodeAnalyzer,
    DenoInNpmPackageChecker,
    DenoIsBuiltInNodeModuleChecker,
    NpmPackageFolderResolverImpl,
    RealSys,
>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CjsAnalysis {
    Esm,
    EsmAnalysis(ModuleExportsAndReExports),
    Cjs(ModuleExportsAndReExports),
}

impl From<ExtNodeCjsAnalysis<'_>> for CjsAnalysis {
    fn from(analysis: ExtNodeCjsAnalysis) -> Self {
        match analysis {
            ExtNodeCjsAnalysis::Esm(_, Some(exports)) => {
                CjsAnalysis::EsmAnalysis(ModuleExportsAndReExports {
                    exports: exports.exports,
                    reexports: exports.reexports,
                })
            }
            ExtNodeCjsAnalysis::Esm(_, None) => CjsAnalysis::Esm,
            ExtNodeCjsAnalysis::Cjs(analysis) => CjsAnalysis::Cjs(ModuleExportsAndReExports {
                exports: analysis.exports,
                reexports: analysis.reexports,
            }),
        }
    }
}

pub struct CjsCodeAnalyzer {
    fs: deno_fs::FileSystemRc,
    cache: std::cell::RefCell<FxHashMap<String, CjsAnalysis>>,
    cjs_tracker: Arc<Resolver>,
}

impl CjsCodeAnalyzer {
    pub fn new(fs: deno_fs::FileSystemRc, cjs_tracker: Arc<Resolver>) -> Self {
        Self { fs, cache: std::cell::RefCell::new(FxHashMap::default()), cjs_tracker }
    }

    async fn inner_cjs_analysis(
        &self,
        specifier: &ModuleSpecifier,
        source: &str,
        esm_analysis_mode: EsmAnalysisMode,
    ) -> Result<CjsAnalysis, JsErrorBox> {
        if let Some(analysis) = self.cache.borrow().get(specifier.as_str()) {
            return Ok(analysis.clone());
        }

        let source = source.strip_prefix('\u{FEFF}').unwrap_or(source);
        let media_type = MediaType::from_specifier(specifier);

        if media_type == MediaType::Json {
            return Ok(CjsAnalysis::Cjs(Default::default()));
        }

        let cjs_tracker = self.cjs_tracker.clone();
        let specifier_clone = specifier.clone();

        let analysis = deno_core::unsync::spawn_blocking({
            let source: Arc<str> = source.into();
            move || -> Result<CjsAnalysis, JsErrorBox> {
                let parsed_source = deno_ast::parse_program(deno_ast::ParseParams {
                    specifier: specifier_clone.clone(),
                    text: source,
                    media_type,
                    capture_tokens: true,
                    scope_analysis: false,
                    maybe_syntax: None,
                })
                .map_err(JsErrorBox::from_err)?;

                let is_script = parsed_source.compute_is_script();
                let is_cjs = cjs_tracker.is_cjs(parsed_source.specifier(), media_type, is_script);

                if is_cjs {
                    Ok(CjsAnalysis::Cjs(ModuleExportsAndReExports {
                        exports: vec![],
                        reexports: vec![],
                    }))
                } else {
                    match esm_analysis_mode {
                        EsmAnalysisMode::SourceOnly => Ok(CjsAnalysis::Esm),
                        EsmAnalysisMode::SourceImportsAndExports => {
                            Ok(CjsAnalysis::EsmAnalysis(ModuleExportsAndReExports {
                                exports: vec![],
                                reexports: vec![],
                            }))
                        }
                    }
                }
            }
        })
        .await
        .expect("task panicked")?;

        self.cache.borrow_mut().insert(specifier.as_str().to_string(), analysis.clone());
        Ok(analysis)
    }

    #[allow(unused)]
    fn analyze_cjs<'a>(
        &self,
        specifier: &ModuleSpecifier,
        source: Cow<'a, str>,
        esm_analysis_mode: EsmAnalysisMode,
    ) -> Result<ExtNodeCjsAnalysis<'a>, JsErrorBox> {
        let rt = tokio::runtime::Handle::current();
        let analysis =
            rt.block_on(self.inner_cjs_analysis(specifier, &source, esm_analysis_mode))?;

        match analysis {
            CjsAnalysis::Esm => Ok(ExtNodeCjsAnalysis::Esm(source, None)),
            CjsAnalysis::EsmAnalysis(analysis) => Ok(ExtNodeCjsAnalysis::Esm(
                source,
                Some(CjsAnalysisExports {
                    exports: analysis.exports,
                    reexports: analysis.reexports,
                }),
            )),
            CjsAnalysis::Cjs(analysis) => Ok(ExtNodeCjsAnalysis::Cjs(CjsAnalysisExports {
                exports: analysis.exports,
                reexports: analysis.reexports,
            })),
        }
    }
}

#[async_trait::async_trait(?Send)]
impl node_resolver::analyze::CjsCodeAnalyzer for CjsCodeAnalyzer {
    async fn analyze_cjs<'a>(
        &self,
        specifier: &ModuleSpecifier,
        source: Option<Cow<'a, str>>,
        esm_analysis_mode: EsmAnalysisMode,
    ) -> Result<ExtNodeCjsAnalysis<'a>, JsErrorBox> {
        let source = match source {
            Some(source) => source,
            None => {
                if let Ok(path) = specifier.to_file_path() {
                    if let Ok(source_from_file) = self
                        .fs
                        .read_text_file_lossy_async(deno_permissions::CheckedPathBuf::unsafe_new(
                            path,
                        ))
                        .await
                    {
                        source_from_file
                    } else {
                        return Ok(ExtNodeCjsAnalysis::Cjs(CjsAnalysisExports {
                            exports: vec![],
                            reexports: vec![],
                        }));
                    }
                } else {
                    return Ok(ExtNodeCjsAnalysis::Cjs(CjsAnalysisExports {
                        exports: vec![],
                        reexports: vec![],
                    }));
                }
            }
        };

        let analysis = self.inner_cjs_analysis(specifier, &source, esm_analysis_mode).await?;

        match analysis {
            CjsAnalysis::Esm => Ok(ExtNodeCjsAnalysis::Esm(source, None)),
            CjsAnalysis::EsmAnalysis(analysis) => Ok(ExtNodeCjsAnalysis::Esm(
                source,
                Some(CjsAnalysisExports {
                    exports: analysis.exports,
                    reexports: analysis.reexports,
                }),
            )),
            CjsAnalysis::Cjs(analysis) => Ok(ExtNodeCjsAnalysis::Cjs(CjsAnalysisExports {
                exports: analysis.exports,
                reexports: analysis.reexports,
            })),
        }
    }
}
