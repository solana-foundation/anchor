use anyhow::anyhow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use syn::parse::{Error as ParseError, Result as ParseResult};

/// Crate parse context
///
/// Keeps track of modules defined within a crate.
pub struct CrateContext {
    modules: BTreeMap<String, ParsedModule>,
}

impl CrateContext {
    pub fn consts(&self) -> impl Iterator<Item = &syn::ItemConst> {
        self.modules.iter().flat_map(|(_, ctx)| ctx.consts())
    }

    pub fn structs(&self) -> impl Iterator<Item = &syn::ItemStruct> {
        self.modules.iter().flat_map(|(_, ctx)| ctx.structs())
    }

    pub fn enums(&self) -> impl Iterator<Item = &syn::ItemEnum> {
        self.modules.iter().flat_map(|(_, ctx)| ctx.enums())
    }

    pub fn modules(&self) -> impl Iterator<Item = ModuleContext> {
        self.modules
            .iter()
            .map(move |(_, detail)| ModuleContext { detail })
    }

    pub fn root_module(&self) -> ModuleContext {
        ModuleContext {
            detail: self.modules.get("crate").unwrap(),
        }
    }

    pub fn parse(root: impl AsRef<Path>) -> Result<Self, anyhow::Error> {
        Ok(CrateContext {
            modules: ParsedModule::parse_recursive(root.as_ref())?,
        })
    }

    // Perform Anchor safety checks on the parsed create
    pub fn safety_checks(&self) -> Result<(), anyhow::Error> {
        // Check all structs for unsafe field types, i.e. AccountInfo and UncheckedAccount.
        for (_, ctx) in self.modules.iter() {
            for unsafe_field in ctx.unsafe_struct_fields() {
                // Check if unsafe field type has been documented with a /// SAFETY: doc string.
                let is_documented = unsafe_field.attrs.iter().any(|attr| {
                    attr.tokens.clone().into_iter().any(|token| match token {
                        // Check for doc comments containing CHECK
                        proc_macro2::TokenTree::Literal(s) => s.to_string().contains("CHECK"),
                        _ => false,
                    })
                });
                if !is_documented {
                    let ident = unsafe_field.ident.as_ref().unwrap();
                    let span = ident.span();
                    // Error if undocumented.
                    return Err(anyhow!(
                        r#"
        {}:{}:{}
        Struct field "{}" is unsafe, but is not documented.
        Please add a `/// CHECK:` doc comment explaining why no checks through types are necessary.
        See https://book.anchor-lang.com/anchor_in_depth/the_accounts_struct.html#safety-checks for more information.
                    "#,
                        ctx.file.canonicalize().unwrap().display(),
                        span.start().line,
                        span.start().column,
                        ident.to_string()
                    ));
                };
            }
        }
        Ok(())
    }
}

/// Module parse context
///
/// Keeps track of items defined within a module.
#[derive(Copy, Clone)]
pub struct ModuleContext<'krate> {
    detail: &'krate ParsedModule,
}

impl<'krate> ModuleContext<'krate> {
    pub fn items(&self) -> impl Iterator<Item = &syn::Item> {
        self.detail.items.iter()
    }
}
struct ParsedModule {
    name: String,
    file: PathBuf,
    path: String,
    items: Vec<syn::Item>,
}

impl ParsedModule {
    fn parse_recursive(root: &Path) -> Result<BTreeMap<String, ParsedModule>, anyhow::Error> {
        let mut modules = BTreeMap::new();

        let root_content = std::fs::read_to_string(root)?;
        let root_file = syn::parse_file(&root_content)?;
        let root_mod = Self::new(
            String::new(),
            root.to_owned(),
            "crate".to_owned(),
            root_file.items,
        );

        struct UnparsedModule {
            file: PathBuf,
            path: String,
            name: String,
            item: syn::ItemMod,
        }

        let mut unparsed = root_mod
            .submodules()
            .map(|item| UnparsedModule {
                file: root_mod.file.clone(),
                path: root_mod.path.clone(),
                name: item.ident.to_string(),
                item: item.clone(),
            })
            .collect::<Vec<_>>();

        while let Some(to_parse) = unparsed.pop() {
            let path = format!("{}::{}", to_parse.path, to_parse.name);
            let name = to_parse.name;
            let module = Self::from_item_mod(&to_parse.file, &path, to_parse.item)?;

            unparsed.extend(module.submodules().map(|item| UnparsedModule {
                item: item.clone(),
                file: module.file.clone(),
                path: module.path.clone(),
                name: item.ident.to_string(),
            }));
            modules.insert(format!("{}{}", module.path.clone(), name.clone()), module);
        }

        modules.insert(root_mod.name.clone(), root_mod);

        Ok(modules)
    }

    fn from_item_mod(
        parent_file: &Path,
        parent_path: &str,
        item: syn::ItemMod,
    ) -> ParseResult<Self> {
        Ok(match item.content {
            Some((_, items)) => {
                // The module content is within the parent file being parsed
                Self::new(
                    parent_path.to_owned(),
                    parent_file.to_owned(),
                    item.ident.to_string(),
                    items,
                )
            }
            None => {
                // The module is referencing some other file, so we need to load that
                // to parse the items it has.
                let parent_dir = parent_file.parent().unwrap();
                let parent_filename = parent_file.file_stem().unwrap().to_str().unwrap();
                let parent_mod_dir = parent_dir.join(parent_filename);

                let possible_file_paths = vec![
                    parent_dir.join(format!("{}.rs", item.ident)),
                    parent_dir.join(format!("{}/mod.rs", item.ident)),
                    parent_mod_dir.join(format!("{}.rs", item.ident)),
                    parent_mod_dir.join(format!("{}/mod.rs", item.ident)),
                ];

                let mod_file_path = possible_file_paths
                    .into_iter()
                    .find(|p| p.exists())
                    .ok_or_else(|| ParseError::new_spanned(&item, "could not find file"))?;
                let mod_file_content = std::fs::read_to_string(&mod_file_path)
                    .map_err(|_| ParseError::new_spanned(&item, "could not read file"))?;
                let mod_file = syn::parse_file(&mod_file_content)?;

                Self::new(
                    parent_path.to_owned(),
                    mod_file_path,
                    item.ident.to_string(),
                    mod_file.items,
                )
            }
        })
    }

    fn new(path: String, file: PathBuf, name: String, items: Vec<syn::Item>) -> Self {
        Self {
            name,
            file,
            path,
            items,
        }
    }

    fn submodules(&self) -> impl Iterator<Item = &syn::ItemMod> {
        self.items.iter().filter_map(|i| match i {
            syn::Item::Mod(item) => Some(item),
            _ => None,
        })
    }

    fn structs(&self) -> impl Iterator<Item = &syn::ItemStruct> {
        self.items.iter().filter_map(|i| match i {
            syn::Item::Struct(item) => Some(item),
            _ => None,
        })
    }

    fn unsafe_struct_fields(&self) -> impl Iterator<Item = &syn::Field> {
        self.structs()
            .flat_map(|s| &s.fields)
            .filter(|f| match &f.ty {
                syn::Type::Path(syn::TypePath {
                    path: syn::Path { segments, .. },
                    ..
                }) => {
                    segments.len() == 1 && segments[0].ident == "UncheckedAccount"
                        || segments[0].ident == "AccountInfo"
                }
                _ => false,
            })
    }

    fn enums(&self) -> impl Iterator<Item = &syn::ItemEnum> {
        self.items.iter().filter_map(|i| match i {
            syn::Item::Enum(item) => Some(item),
            _ => None,
        })
    }

    fn consts(&self) -> impl Iterator<Item = &syn::ItemConst> {
        self.items.iter().filter_map(|i| match i {
            syn::Item::Const(item) => Some(item),
            _ => None,
        })
    }
}
