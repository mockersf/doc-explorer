use chromadb::v2::{collection::QueryOptions, ChromaClient};

use crate::ollama::SimpleOllama;

pub async fn retrieve(
    ollama: SimpleOllama,
    collection_name: &str,
    prompt: &str,
) -> Result<Vec<(String, f32)>, Box<dyn std::error::Error>> {
    let chroma = ChromaClient::new(Default::default());

    let collection = chroma.get_collection(&collection_name).await?;

    let embeddings = ollama.embeddings(prompt).await?;
    let query = QueryOptions {
        query_embeddings: Some(vec![embeddings]),
        n_results: Some(10),
        include: Some(vec!["distances"]),
        ..Default::default()
    };
    let result = collection.query(query, None).await?;
    Ok(result.ids[0]
        .iter()
        .enumerate()
        .map(|(i, doc)| {
            let mut doc = doc.clone();
            let _ = doc.split_off(doc.len() - 3);
            (doc, result.distances.as_ref().unwrap()[0][i])
        })
        .collect())
}
