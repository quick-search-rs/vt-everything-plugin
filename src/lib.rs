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
    config: quick_search_lib::Config,
}

impl Everything {
    fn new(id: PluginId) -> Self {
        let mut config = quick_search_lib::Config::default();
        config.insert(
            "Max Results".into(),
            quick_search_lib::EntryType::Int {
                value: 50,
                min: Some(1).into(),
                max: Some(250).into(),
            },
        );
        Self { id, config }
    }
}

impl Searchable for Everything {
    fn search(&self, query: RString) -> RVec<SearchResult> {
        let mut res: Vec<SearchResult> = vec![];

        let max_results = self.config.get("Max Results").and_then(|v| v.as_int()).unwrap_or(50) as u32; // extras in case of duplicates

        if let Ok(query_as_wchar) = U16CString::from_str(query) {
            unsafe {
                Everything_SetSearchW(query_as_wchar.as_ptr());
            }
            if unsafe { Everything_QueryW(1) } == 1 {
                let f = unsafe { Everything_GetNumResults() };
                println!("found {} results", f);
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

                    // do not add duplicates
                    let result = SearchResult::new(&fullfile).set_context(&format!("{}\\{}", path, fullfile));
                    if res.contains(&result) {
                        continue;
                    } else {
                        res.push(result);
                        if res.len() >= max_results as usize {
                            break;
                        }
                    }
                }
            }
        } else {
            eprintln!("failed to convert query to wchar");
        }

        res.sort_by(|a, b| a.title().cmp(b.title()));
        res.dedup();
        res.truncate(max_results as usize);

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
        //     eprintln!("failed to copy to clipboard: {}", s);
        // }

        // finish up, above is a clipboard example
        println!("opening file: {}", result.context());

        let path = {
            match std::path::PathBuf::from_str(result.context()) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("failed to get path: {}", e);
                    return;
                }
            }
        };

        println!("path: {:?}", path);

        if let Err(e) = opener::open(path) {
            eprintln!("failed to open file: {}", e);
        } else {
            println!("opened file");
        }
    }
    fn plugin_id(&self) -> PluginId {
        self.id.clone()
    }
    fn get_config_entries(&self) -> quick_search_lib::Config {
        self.config.clone()
    }
    fn lazy_load_config(&mut self, config: quick_search_lib::Config) {
        self.config = config;
        unsafe {
            Everything_SetMax(self.config.get("Max Results").and_then(|v| v.as_int()).unwrap_or(50) as u32 * 4);
            // some extras in case of duplicates, should never run into memory allocation issues
        }
    }
}
