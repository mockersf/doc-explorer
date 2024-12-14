use std::collections::HashSet;
use std::io::Write;

type CrateCatalog = [Option<(String, rustdoc_types::Crate)>];

pub fn generate_docs(entry: String) {
    std::fs::create_dir_all("docs").unwrap();
    let Ok(json_string) = std::fs::read_to_string(format!("./jsons/{}.json", entry)) else {
        println!("Couldn't find {}.json", entry);
        println!(
                "You should generate all jsons from rustdoc and place them in the jsons directory by running the following commands:"
            );
        println!("You can run the following command in the project you want to document:");
        println!();
        println!("> RUSTDOCFLAGS=\"-Z unstable-options --output-format json\" cargo +nightly doc");
        println!();
        println!(
                "then move the generated jsons from target/doc/ to the jsons directory in the rustdoc-rag project"
            );
        panic!()
    };
    let krate: rustdoc_types::Crate = serde_json::from_str(&json_string).unwrap();

    let mut loaded_crates = vec![None; krate.external_crates.len() + 1];

    for ext_krate in &krate.external_crates {
        if ext_krate.1.name == "typenum" {
            continue;
        }
        let Ok(json_string) = std::fs::read_to_string(format!("./jsons/{}.json", ext_krate.1.name))
        else {
            continue;
        };
        let Ok(krate) = serde_json::from_str(&json_string) else {
            panic!("Couldn't parse json for crate {}", ext_krate.1.name);
        };
        loaded_crates[*ext_krate.0 as usize] = Some((ext_krate.1.name.clone(), krate));
    }
    loaded_crates[0] = Some(("bevy".to_string(), krate));

    let mut visited = HashSet::<(usize, rustdoc_types::Id)>::new();
    start_krate(&loaded_crates, &mut visited);
}

fn start_krate(crates: &CrateCatalog, visited: &mut HashSet<(usize, rustdoc_types::Id)>) {
    let krate = &crates[0].as_ref().unwrap().1;
    item_explorer(krate.root, 0, crates, visited, 0);
}

fn item_explorer(
    id: rustdoc_types::Id,
    current_crate: usize,
    crates: &CrateCatalog,
    visited: &mut HashSet<(usize, rustdoc_types::Id)>,
    depth: u32,
) {
    if !visited.insert((current_crate, id)) {
        return;
    }
    let krate = crates[current_crate].as_ref().unwrap();
    let item = if let Some(item) = krate.1.index.get(&id) {
        item
    } else {
        krate.1.index.get(&krate.1.root).unwrap()
    };
    match &item.inner {
        rustdoc_types::ItemEnum::Module(module) => {
            module_explorer(module, current_crate, crates, visited, depth);
        }
        rustdoc_types::ItemEnum::ExternCrate { .. } => todo!(),
        rustdoc_types::ItemEnum::Use(used) => {
            let crate_name = used.source.split("::").next().unwrap();
            if crate_name == "crate" || crate_name == "super" {
                return item_explorer(used.id.unwrap(), current_crate, crates, visited, depth + 1);
            }
            for (crate_index, krate) in crates.iter().enumerate() {
                if let Some(krate) = krate {
                    if krate.0 == crate_name {
                        return item_explorer(
                            rustdoc_types::Id(u32::MAX),
                            crate_index,
                            crates,
                            visited,
                            depth + 1,
                        );
                    }
                }
            }
            return item_explorer(used.id.unwrap(), current_crate, crates, visited, depth + 1);
        }
        rustdoc_types::ItemEnum::Union(_union) => todo!(),
        rustdoc_types::ItemEnum::Struct(stru) => {
            document_struct(item, stru, current_crate, crates);
        }
        rustdoc_types::ItemEnum::StructField(_strufi) => {}
        rustdoc_types::ItemEnum::Enum(enume) => {
            enum_explorer(enume, current_crate, crates, visited, depth);
        }
        rustdoc_types::ItemEnum::Variant(_) => {}
        rustdoc_types::ItemEnum::Function(_) => {}
        rustdoc_types::ItemEnum::Trait(_) => {}
        rustdoc_types::ItemEnum::TraitAlias(_) => todo!(),
        rustdoc_types::ItemEnum::Impl(_) => {}
        rustdoc_types::ItemEnum::TypeAlias(_) => {}
        rustdoc_types::ItemEnum::Constant { .. } => {}
        rustdoc_types::ItemEnum::Static(_) => {}
        rustdoc_types::ItemEnum::ExternType => todo!(),
        rustdoc_types::ItemEnum::Macro(_) => {}
        rustdoc_types::ItemEnum::ProcMacro(_proc_macro) => {}
        rustdoc_types::ItemEnum::Primitive(_primitive) => todo!(),
        rustdoc_types::ItemEnum::AssocConst { .. } => todo!(),
        rustdoc_types::ItemEnum::AssocType { .. } => {}
    }
}

fn module_explorer(
    module: &rustdoc_types::Module,
    current_crate: usize,
    crates: &CrateCatalog,
    visited: &mut HashSet<(usize, rustdoc_types::Id)>,
    depth: u32,
) {
    for item in &module.items {
        item_explorer(*item, current_crate, crates, visited, depth + 1);
    }
}

fn enum_explorer(
    enumeration: &rustdoc_types::Enum,
    current_crate: usize,
    crates: &CrateCatalog,
    visited: &mut HashSet<(usize, rustdoc_types::Id)>,
    depth: u32,
) {
    enumeration.variants.iter().for_each(|variant| {
        item_explorer(*variant, current_crate, crates, visited, depth + 1);
    });
}

struct StructDocument {
    name: String,
    docs: Option<String>,
    fields: Vec<Field>,
}

struct Field {
    name: String,
    docs: Option<String>,
}

pub fn document_struct(
    item: &rustdoc_types::Item,
    stru: &rustdoc_types::Struct,
    current_crate: usize,
    crates: &CrateCatalog,
) {
    std::fs::create_dir_all("docs/structs").unwrap();
    let mut doc = StructDocument {
        name: item.name.as_ref().unwrap().to_string(),
        docs: item.docs.clone(),
        fields: vec![],
    };

    match &stru.kind {
        rustdoc_types::StructKind::Unit => {}
        rustdoc_types::StructKind::Tuple(_fields) => {}
        rustdoc_types::StructKind::Plain { fields, .. } => {
            doc.fields = fields
                .iter()
                .map(|field| {
                    let field = crates
                        .get(current_crate)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .1
                        .index
                        .get(field)
                        .unwrap();
                    Field {
                        name: field.name.as_ref().unwrap().to_string(),
                        docs: field.docs.clone(),
                    }
                })
                .collect();
        }
    }
    doc.write();
}

impl StructDocument {
    pub fn write(&self) {
        let mut file = std::fs::File::create(format!("docs/structs/{}.md", self.name)).unwrap();

        write!(file, "{} is a struct.\n\n", self.name).unwrap();
        if let Some(docs) = &self.docs {
            write!(file, "{}\n\n", docs).unwrap();
        }
        if !self.fields.is_empty() {
            write!(file, "It has the following fields: ").unwrap();
            for field in &self.fields {
                write!(file, "{}, ", field.name).unwrap();
            }
            write!(file, "\n\n").unwrap();

            for field in &self.fields {
                if let Some(docs) = &field.docs {
                    write!(file, "More details about the {} field:\n\n", field.name).unwrap();
                    write!(file, "{}\n\n", docs).unwrap();
                }
            }
        }
    }
}
