use std::error::Error;

use ollama_rs::{generation::embeddings::request::GenerateEmbeddingsRequest, Ollama};

pub struct SimpleOllama {
    ollama: Ollama,
    embedding_model: String,
}

impl SimpleOllama {
    pub fn new(embedding_model: String) -> Self {
        SimpleOllama {
            ollama: Ollama::default(),
            embedding_model,
        }
    }

    pub async fn download_model(&self) -> Result<(), Box<dyn Error>> {
        let Ok(models) = self.ollama.list_local_models().await else {
            println!("Error generating embeddings");
            println!("Is Ollama running?");
            panic!();
        };

        for model in models {
            if model.name == self.embedding_model {
                return Ok(());
            }
        }

        println!("downloading model {}", self.embedding_model);
        self.ollama
            .pull_model(self.embedding_model.clone(), false)
            .await?;

        Ok(())
    }

    pub async fn embeddings(&self, document: &str) -> Result<Vec<f32>, Box<dyn Error>> {
        let request = GenerateEmbeddingsRequest::new(self.embedding_model.clone(), document.into());
        let mut res = self.ollama.generate_embeddings(request).await?;
        Ok(res.embeddings.remove(0))
    }
}
