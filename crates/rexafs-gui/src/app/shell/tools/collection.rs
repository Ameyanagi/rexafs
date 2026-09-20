//! Prepare every explicitly selected analysis operand. Plot sampling must not
//! silently determine the scientific training set or reference library.

use super::*;

impl StudioApp {
    pub(super) fn analysis_selection(&self, tool: Tool) -> BTreeSet<usize> {
        if tool == Tool::Lcf {
            let current = self.current_group_index().and_then(|ix| self.group_id(ix));
            analysis_marks(&self.selection, current.as_ref(), |ix| self.group_id(ix))
        } else {
            self.selection.clone()
        }
    }

    /// True means a background load is pending. The calculation is retried
    /// only if identities, processing, selection and form settings still match.
    pub(super) fn prepare_analysis_collection(
        &mut self,
        tool: Tool,
        cx: &mut Context<Self>,
    ) -> Result<bool, String> {
        if self.analysis.collection_loading {
            return Err("Wait for analysis inputs to finish loading".into());
        }
        let marks = self.analysis_selection(tool);
        let indices: Vec<_> = marked_group_indices(&marks).collect();
        let required = indices.len() + usize::from(tool == Tool::Lcf);
        if required > self.cache.cap().get() {
            return Err(format!(
                "This analysis needs {} in memory; the current limit is {}. Select fewer inputs.",
                crate::text::plural(required, "spectrum"),
                self.cache.cap()
            ));
        }
        let pending = crate::app::missing_compare_loads(
            &self.catalog,
            &self.derived,
            &self.params,
            &self.overrides,
            &indices,
            &self.cache,
        );
        if pending.is_empty() {
            if crate::app::cached_marked_indices(&marks, &self.cache, |ix| {
                self.effective_fingerprint(ix)
            })
            .count()
                != marks.len()
            {
                return Err(
                    "A selected spectrum is unavailable; no partial analysis was run".into(),
                );
            }
            return Ok(false);
        }
        let sources = indices
            .iter()
            .map(|&ix| {
                self.tool_target(ix)
                    .ok_or("A selected spectrum is unavailable")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let target = self.current_tool_target();
        let settings = (
            self.tools.pca_config(),
            self.tools.lcf_sum_to_one,
            self.tools.lcf_e0_shift,
            self.tools.lcf_all_combinations,
            self.tools.pca_components,
        );
        self.analysis.collection_generation += 1;
        let generation = self.analysis.collection_generation;
        self.analysis.collection_loading = true;
        self.tools.message = format!(
            "Loading {} selected {}…",
            pending.len(),
            crate::text::noun_for(pending.len(), "spectrum")
        )
        .into();
        let job = cx.background_executor().spawn(async move {
            pending
                .into_iter()
                .map(|load| load.process())
                .collect::<Vec<_>>()
        });
        cx.spawn(async move |this,cx| {
            let loaded=job.await;
            this.update(cx,|app,cx| {
                if app.analysis.collection_generation!=generation { return; }
                app.analysis.collection_loading=false;
                if let Err(error) = app.tools.sync_range(cx) {
                    app.tools.message = error.into(); cx.notify(); return;
                }
                let unchanged=app.tools.open==Some(tool) && target==app.current_tool_target() && marks==app.analysis_selection(tool)
                    && settings==(app.tools.pca_config(),app.tools.lcf_sum_to_one,app.tools.lcf_e0_shift,
                        app.tools.lcf_all_combinations,app.tools.pca_components)
                    && sources.iter().all(|s| app.tool_target(s.ix).as_ref()==Some(s));
                if !unchanged {
                    app.tools.message="Analysis inputs or settings changed; run again with the current selection.".into();
                    cx.notify(); return;
                }
                for (ix,fingerprint,result) in loaded {
                    match result {
                        Ok((spectrum,_)) => { app.cache.put((ix,fingerprint),Arc::new(spectrum)); }
                        Err(error) => {
                            app.tools.message=format!("{}: {error}; no partial analysis was run",app.entry_label(ix)).into();
                            cx.notify(); return;
                        }
                    }
                }
                let _=app.run_analysis_tool(tool,cx);
                cx.notify();
            }).ok();
        }).detach();
        cx.notify();
        Ok(true)
    }
}
