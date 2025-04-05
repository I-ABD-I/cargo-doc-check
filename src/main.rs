use std::fs;
use std::path::{Path, PathBuf};
use cargo_metadata::MetadataCommand;
use syn::{ImplItem, Item, ItemTrait};
use syn::Attribute;
use syn::spanned::Spanned;
use syn::visit::Visit;
use walkdir::WalkDir;
use owo_colors::*;
use proc_macro2::LineColumn;

fn has_doc(attrs:&[Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("doc"))
}

struct DocChecker<'a> {
    curr_file: &'a Path,
}

fn print_warning(name: &str, file: &Path, location: &LineColumn) {
    println!("{}: Missing doc for `{name}`\n  {} {}:{}:{}", "warning".yellow().bold(), "-->".bright_blue().bold(), file.display(), location.line, location.column );
}

impl Visit<'_> for DocChecker<'_> {
    fn visit_impl_item(&mut self, i: &'_ ImplItem) {
        let (name, attrs) = match i {
            ImplItem::Fn(f) => (f.sig.ident.to_string(), &f.attrs),
            ImplItem::Const(c) => (c.ident.to_string(), &c.attrs),
            _ => return
        };

        if !has_doc(attrs) {
            print_warning(&name, self.curr_file, &i.span().start());
        }
    }

    fn visit_item(&mut self, i: &'_ Item) {
        let (name, attrs ) = match i {
            Item::Fn(f) => (f.sig.ident.to_string(), &f.attrs),
            Item::Struct(s) => (s.ident.to_string(), &s.attrs),
            Item::Enum(e) => (e.ident.to_string(), &e.attrs),
            Item::Trait(t) => (t.ident.to_string(), &t.attrs),
            Item::Impl(_) => return syn::visit::visit_item(self, i),
            Item::Const(c) => (c.ident.to_string(), &c.attrs),
            _ => return,
        };

        if !has_doc(attrs) {
            print_warning(&name, self.curr_file, &i.span().start());
        }

        // Recursively visit nested items
        syn::visit::visit_item(self, i);
    }

    fn visit_item_trait(&mut self, i: &'_ ItemTrait) {
        let (name, attrs) = (i.ident.to_string(), &i.attrs);

        if !has_doc(attrs) {
            print_warning(&name, self.curr_file, &i.span().start());
        }

        // Recursively visit nested items
        syn::visit::visit_item_trait(self, i);
    }
}

fn scan_sub_crate(path: &Path) {
    // Iterate over all Rust files in the path
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let path = entry.path();
        let content = fs::read_to_string(path).unwrap();
        let parsed: syn::File = syn::parse_file(&content).unwrap_or_else(|err| {
            panic!("Failed to parse {:?}: {}", path, err);
        });

        // Run DocChecker logic here
        DocChecker { curr_file: path }.visit_file(&parsed);
    }
}

fn main() {
    let metadata = MetadataCommand::new().exec().unwrap();
    for package in metadata.packages.iter().filter(|p| metadata.workspace_members.contains(&p.id)) {
        scan_sub_crate(&PathBuf::from(&package.manifest_path).parent().unwrap().join("src"))
    }
}