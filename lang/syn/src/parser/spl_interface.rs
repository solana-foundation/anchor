#[cfg(feature = "interface-instructions")]
use syn::{Meta, NestedMeta, Path};

#[cfg(not(feature = "interface-instructions"))]
pub fn parse(_attrs: &[syn::Attribute]) -> Option<[u8; 8]> {
    None
}

#[cfg(feature = "interface-instructions")]
pub fn parse(attrs: &[syn::Attribute]) -> Option<[u8; 8]> {
    let interfaces: Vec<[u8; 8]> = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path.is_ident("interface") {
                if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                    if let Some(NestedMeta::Meta(Meta::Path(path))) = meta_list.nested.first() {
                        return Some(parse_interface_instruction(path));
                    }
                }
                panic!(
                    "Failed to parse interface instruction:\n{}",
                    quote::quote!(#attr)
                );
            }
            None
        })
        .collect();
    if interfaces.len() > 1 {
        panic!("An instruction can only implement one interface instruction");
    } else if interfaces.is_empty() {
        None
    } else {
        Some(interfaces[0])
    }
}

#[cfg(feature = "interface-instructions")]
fn parse_interface_instruction(path: &Path) -> [u8; 8] {
    if path.segments.len() != 2 {
        // All interface instruction args are expected to be in the form
        // <interface>::<instruction>
        panic!(
            "Invalid interface instruction: {}",
            path.segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<String>>()
                .join("::")
        );
    }
    let interface = path.segments[0].ident.to_string();
    if interface == "spl_transfer_hook_interface" {
        let instruction = path.segments[1].ident.to_string();
        if instruction == "initialize_extra_account_meta_list" {
            return [43, 34, 13, 49, 167, 88, 235, 235]; // `InitializeExtraAccountMetaList`
        } else if instruction == "execute" {
            return [105, 37, 101, 197, 75, 251, 102, 26]; // `Execute`
        } else {
            panic!("Unsupported instruction: {}", instruction);
        }
    }
    panic!("Unsupported interface: {}", interface);
}
