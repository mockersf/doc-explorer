use chromadb::v2::{collection::CollectionEntries, ChromaClient};
use serde_json::Map;

use crate::ollama::SimpleOllama;

pub async fn generate_embeddings(
    ollama: SimpleOllama,
    collection_name: &str,
    distance: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let chroma: ChromaClient = ChromaClient::new(Default::default());

    let mut collection_meta = Map::new();
    collection_meta.insert("hnsw:space".to_string(), distance.into());
    let collection = chroma
        .get_or_create_collection(&collection_name, Some(collection_meta))
        .await?;

    let dir = std::fs::read_dir("./docs/structs")?;
    for entry in dir {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        let entries = CollectionEntries {
            ids: vec![file_name],
            embeddings: Some(vec![
                ollama.embeddings(&std::fs::read_to_string(&path)?).await?,
            ]),
            ..Default::default()
        };
        collection.upsert(entries, None).await?;
    }
    Ok(())
}
