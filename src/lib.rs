use std::str::FromStr;

use abi_stable::{
    export_root_module,
    prefix_type::PrefixTypeTrait,
    sabi_extern_fn,
    sabi_trait::prelude::TD_Opaque,
    std_types::{RBox, RStr, RString, RVec},
};
use everything_sys::*;
use quick_search_lib::{ColoredChar, PluginId, SearchLib, SearchLib_Ref, SearchResult, Searchable, Searchable_TO};
use widestring::U16CString;

static NAME: &str = "Everything";

#[export_root_module]
pub fn get_library() -> SearchLib_Ref {
    SearchLib { get_searchable }.leak_into_prefix()
}

#[sabi_extern_fn]
fn get_searchable(id: PluginId) -> Searchable_TO<'static, RBox<()>> {
    let this = Everything::new(id);
    Searchable_TO::from_value(this, TD_Opaque)
}

#[derive(Debug, Clone)]
struct Everything {
    id: PluginId,
}

impl Everything {
    fn new(id: PluginId) -> Self {
        Self { id }
    }
}

impl Searchable for Everything {
    fn search(&self, query: RString) -> RVec<SearchResult> {
        let mut res: Vec<SearchResult> = vec![];

        if let Ok(query_as_wchar) = U16CString::from_str(query) {
            unsafe {
                Everything_SetSearchW(query_as_wchar.as_ptr());
            }
            if unsafe { Everything_QueryW(1) } == 1 {
                let f = unsafe { Everything_GetNumResults() }.clamp(0, 100);
                for i in 0..f {
                    let filename = unsafe {
                        let ptr = Everything_GetResultFileNameW(i);
                        if ptr.is_null() {
                            continue;
                        } else {
                            U16CString::from_ptr_str(ptr).to_string_lossy()
                        }
                    };
                    let extension = unsafe {
                        let ptr = Everything_GetResultExtensionW(i);
                        if !ptr.is_null() {
                            Some(U16CString::from_ptr_str(ptr).to_string_lossy())
                        } else {
                            None
                        }
                    };
                    let path = unsafe {
                        let ptr = Everything_GetResultPathW(i);
                        if ptr.is_null() {
                            continue;
                        } else {
                            U16CString::from_ptr_str(ptr).to_string_lossy()
                        }
                    };
                    // let resstr = format!("{}", filename);
                    let fullfile = match extension {
                        Some(extension) => format!("{}.{}", filename, extension),
                        None => filename.clone(),
                    };

                    // res.push(SearchResult {
                    //     source: Box::new(*self),
                    //     name: fullfile.clone(),
                    //     context: Some(format!("{}\\{}", path, fullfile)),
                    //     action: Some(Box::new(move || {
                    //         // open file

                    //         let path = std::path::PathBuf::from(format!("{}\\{}", path, fullfile));

                    //         super::open(&path);
                    //     })),
                    // });
                    res.push(SearchResult::new(&fullfile).set_context(&format!("{}\\{}", path, fullfile)))
                }
            }
        } else {
            log::error!("failed to convert query to wchar");
        }

        res.sort_by(|a, b| a.title().cmp(b.title()));
        res.dedup_by(|a, b| a.title() == b.title());

        res.into()
    }
    fn name(&self) -> RStr<'static> {
        NAME.into()
    }
    fn colored_name(&self) -> RVec<quick_search_lib::ColoredChar> {
        // can be dynamic although it's iffy how it might be used
        ColoredChar::from_string(NAME, 0xFF7F00FF)
    }
    fn execute(&self, result: &SearchResult) {
        // let s = result.extra_info();
        // if let Ok::<clipboard::ClipboardContext, Box<dyn std::error::Error>>(mut clipboard) = clipboard::ClipboardProvider::new() {
        //     if let Ok(()) = clipboard::ClipboardProvider::set_contents(&mut clipboard, s.to_owned()) {
        //         log::!("copied to clipboard: {}", s);
        //     } else {
        //         log::!("failed to copy to clipboard: {}", s);
        //     }
        // } else {
        //     log::error!("failed to copy to clipboard: {}", s);
        // }

        // finish up, above is a clipboard example
        log::info!("opening file: {}", result.context());

        let path = {
            match std::path::PathBuf::from_str(result.context()) {
                Ok(p) => p,
                Err(e) => {
                    log::error!("failed to get path: {}", e);
                    return;
                }
            }
        };

        log::trace!("path: {:?}", path);

        if let Err(e) = opener::open(path) {
            log::error!("failed to open file: {}", e);
        } else {
            log::info!("opened file");
        }
    }
    fn plugin_id(&self) -> &PluginId {
        &self.id
    }
}
