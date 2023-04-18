use std::io::Cursor;

use anyhow::{anyhow, Result};
use log::{debug, info};
use serde::Deserialize;

use crate::{
    httpclient::run_get_request,
    image::{Image, Layer, LayerMetadata},
};

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ManifestResponse {
    fsLayers: Vec<LayerMetadata>,
}

pub struct Registry {
    url: String,
    auth_url: String,
}

impl Registry {
    pub fn new(url: &str, auth_url: &str) -> Self {
        Self {
            url: url.to_string(),
            auth_url: auth_url.to_string(),
        }
    }

    async fn get_token(&self, image_name: &str) -> Result<String> {
        let res = run_get_request(
            &format!(
                "{}?service=registry.docker.io&scope=repository:{}:pull",
                self.auth_url, image_name
            ),
            None,
        )
        .await?;

        // Extract the token from the response
        let token = res
            .json::<TokenResponse>()
            .await
            .map_err(|e| anyhow!(e).context("Failed to parse response"))?
            .token;

        debug!("Successfully got auth token : {:?}", token);

        Ok(token)
    }

    async fn get_layers_metadata(
        &self,
        token: &str,
        image_name: &str,
        image_tag: &str,
    ) -> Result<Vec<LayerMetadata>> {
        let res = run_get_request(
            &format!("{}/{}/manifests/{}", self.url, image_name, image_tag),
            Some(token),
        )
        .await?;

        // Extract the information about the layers from the manifest
        let layers_metadata = res
            .json::<ManifestResponse>()
            .await
            .map_err(|e| anyhow!(e).context("Failed to parse response"))?
            .fsLayers;

        debug!("Successfully got layers : {:?}", layers_metadata);

        Ok(layers_metadata)
    }

    async fn get_layer(
        &self,
        token: &str,
        image_name: &str,
        layer_metadata: &LayerMetadata,
    ) -> Result<Layer> {
        // Make a request to the docker hub to get the layer from his blobSum
        let res = run_get_request(
            &format!(
                "{}/{}/blobs/{}",
                self.url, image_name, layer_metadata.blobSum
            ),
            Some(token),
        )
        .await?;

        // Wrap the response into a tar archive (memory loaded)
        let buff = Cursor::new(
            res.bytes()
                .await
                .map_err(|e| anyhow!(e).context("Failed to read response"))?,
        );

        debug!("Successfully wrote layer to memory");

        Ok(Layer {
            metadata: (*layer_metadata).clone(),
            content: buff,
        })
    }

    async fn get_layers(
        &self,
        token: &str,
        image_name: &str,
        layers_metadata: Vec<LayerMetadata>,
    ) -> Result<Vec<Layer>> {
        let mut layers = Vec::new();
        for layer_metadata in layers_metadata.iter() {
            info!("Pulling layer {:?}", layer_metadata.blobSum);

            let layer = self
                .get_layer(token, image_name, layer_metadata)
                .await
                .map_err(|e| anyhow!(e).context("Failed to pull layer"))?;
            layers.push(layer);

            debug!("Successfully pulled layer : {:?}", layer_metadata.blobSum);
        }

        debug!("Successfully pulled all layers");

        Ok(layers)
    }

    pub async fn get_image(&self, image_full_name: &str) -> Result<Image> {
        let mut image = Image::new(image_full_name)?;
        let image_name = &image.name();
        let token = self.get_token(image_name).await?;
        let layers_metadata = self
            .get_layers_metadata(&token, image_name, image.tag())
            .await?;
        let layers = self.get_layers(&token, image_name, layers_metadata).await?;

        image.layers = layers;

        Ok(image)
    }
}
