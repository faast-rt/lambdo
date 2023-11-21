use std::io::Cursor;

use anyhow::{anyhow, Result};
use log::{debug, info, trace};
use serde::Deserialize;

use crate::{
    httpclient::run_get_request,
    image::{Image, Layer, LayerMetadata},
};

const DEFAULT_PLATFORM_OS: &str = "linux";
const DEFAULT_PLATFORM_ARCHITECTURE: &str = "amd64";

#[derive(Deserialize, Debug)]
struct ManifestListV2Response {
    manifests: Vec<ManifestV2ItemResponse>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ManifestV2Response {
    layers: Vec<LayerInfo>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct OCIv1ListResponse {
    manifests: Vec<LayerInfo>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LayerInfo {
    digest: String,
    platform: Option<Platform>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ManifestV2ItemResponse {
    digest: String,
    platform: Option<Platform>,
}

#[derive(Deserialize, Debug)]
struct Platform {
    architecture: String,
    os: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
}

pub struct Registry {
    url: String,
    auth_url: String,
    token: Option<String>,
}

#[derive(Debug)]
pub enum ManifestType {
    ManifestListV2,
    ManifestV2,
    OCIv1List,
    OCIv1,
}

impl TryFrom<&str> for ManifestType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "application/vnd.docker.distribution.manifest.list.v2+json" => {
                Ok(ManifestType::ManifestListV2)
            }
            "application/vnd.docker.distribution.manifest.v2+json" => Ok(ManifestType::ManifestV2),
            "application/vnd.oci.image.index.v1+json" => Ok(ManifestType::OCIv1List),
            "application/vnd.oci.image.manifest.v1+json" => Ok(ManifestType::OCIv1List),
            _ => Err(anyhow!("Unknown manifest type")),
        }
    }
}

impl ToString for ManifestType {
    fn to_string(&self) -> String {
        match self {
            ManifestType::ManifestListV2 => {
                "application/vnd.docker.distribution.manifest.list.v2+json".to_string()
            }
            ManifestType::ManifestV2 => {
                "application/vnd.docker.distribution.manifest.v2+json".to_string()
            }
            ManifestType::OCIv1List => "application/vnd.oci.image.index.v1+json".to_string(),
            ManifestType::OCIv1 => "application/vnd.oci.image.manifest.v1+json".to_string(),
        }
    }
}

impl Registry {
    pub fn new(url: &str, auth_url: &str) -> Self {
        Self {
            url: url.to_string(),
            auth_url: auth_url.to_string(),
            token: None,
        }
    }

    async fn set_token(&mut self, image_name: &str) -> Result<()> {
        let res = run_get_request(
            &format!(
                "{}?service=registry.docker.io&scope=repository:{}:pull",
                self.auth_url, image_name
            ),
            None,
            Vec::new(),
        )
        .await?;

        // Extract the token from the response
        self.token = Some(
            res.json::<TokenResponse>()
                .await
                .map_err(|e| anyhow!(e).context("Failed to parse response"))?
                .token,
        );

        debug!("Successfully got auth token");

        Ok(())
    }

    async fn get_layers_metadata(
        &self,
        image_name: &str,
        image_tag: &str,
    ) -> Result<Vec<LayerMetadata>> {
        debug!(
            "Url: {}",
            format!("{}/{}/manifests/{}", self.url, image_name, image_tag)
        );

        let headers = vec![
            (
                "Accept".to_string(),
                ManifestType::ManifestListV2.to_string(),
            ),
            ("Accept".to_string(), ManifestType::ManifestV2.to_string()),
        ];

        let res = run_get_request(
            &format!("{}/{}/manifests/{}", self.url, image_name, image_tag),
            self.token.as_ref(),
            headers,
        )
        .await?;

        let content_type = res
            .headers()
            .get("content-type")
            .ok_or(anyhow!("No content type found"))?
            .to_str()
            .map_err(|e| anyhow!(e).context("Failed to parse content type"))?;

        let manifest_type = ManifestType::try_from(content_type).map_err(|e| {
            anyhow!(e).context(format!("Failed to parse content type : {}", content_type))
        })?;

        debug!("Manifest type : {:?}", manifest_type);

        let manifest = match manifest_type {
            ManifestType::ManifestV2 => res
                .json::<ManifestV2Response>()
                .await
                .map_err(|e| anyhow!(e).context("Failed to parse response"))?,
            ManifestType::ManifestListV2 => {
                let manifest = res
                    .json::<ManifestListV2Response>()
                    .await
                    .map_err(|e| anyhow!(e).context("Failed to parse response"))?;

                self.manifest_from_manifest_v2_list(manifest, image_name)
                    .await?
            }
            ManifestType::OCIv1List => {
                let manifest = res
                    .json::<OCIv1ListResponse>()
                    .await
                    .map_err(|e| anyhow!(e).context("Failed to parse response"))?;

                self.manifest_from_oci_list(manifest, image_name).await?
            }
            ManifestType::OCIv1 => unimplemented!(),
        };

        let layers_metadata = manifest
            .layers
            .into_iter()
            .map(|layer| LayerMetadata {
                blob_sum: layer.digest.clone(),
            })
            .collect::<Vec<_>>();

        debug!(
            "Successfully got layers : \n\t{}",
            layers_metadata
                .iter()
                .map(|layer| layer.blob_sum.clone())
                .collect::<Vec<String>>()
                .join("\n\t")
        );

        Ok(layers_metadata)
    }

    async fn manifest_from_manifest_v2_list(
        &self,
        manifest: ManifestListV2Response,
        image_name: &str,
    ) -> Result<ManifestV2Response> {
        let sub_manifest = manifest
            .manifests
            .iter()
            .find(|manifest| {
                manifest
                    .platform
                    .as_ref()
                    .map(|platform| {
                        platform.os == DEFAULT_PLATFORM_OS
                            && platform.architecture == DEFAULT_PLATFORM_ARCHITECTURE
                    })
                    .unwrap_or(false)
            })
            .map(|manifest| manifest.digest.clone())
            .ok_or(anyhow!("No manifest found"))?;

        trace!("Sub manifest : {:?}", sub_manifest);

        self.manifest_from_oci(sub_manifest, image_name).await
    }

    async fn manifest_from_oci_list(
        &self,
        oci_manifest: OCIv1ListResponse,
        image_name: &str,
    ) -> Result<ManifestV2Response> {
        let sub_manifest = oci_manifest
            .manifests
            .into_iter()
            .find(|manifest| {
                manifest
                    .platform
                    .as_ref()
                    .map(|platform| {
                        platform.os == DEFAULT_PLATFORM_OS
                            && platform.architecture == DEFAULT_PLATFORM_ARCHITECTURE
                    })
                    .unwrap_or(false)
            })
            .ok_or(anyhow!("No manifest found"))?;

        trace!("Sub manifest : {:?}", sub_manifest);

        self.manifest_from_oci(sub_manifest.digest, image_name)
            .await
    }

    async fn manifest_from_oci(
        &self,
        oci_manifest: String,
        image_name: &str,
    ) -> Result<ManifestV2Response> {
        debug!(
            "Downloading manifest for {}/{}",
            DEFAULT_PLATFORM_OS, DEFAULT_PLATFORM_ARCHITECTURE
        );

        // Extract the information about the layers from the manifest
        let headers = vec![
            ("Accept".to_string(), ManifestType::OCIv1.to_string()),
            ("Accept".to_string(), ManifestType::ManifestV2.to_string()),
        ];

        let res = run_get_request(
            &format!("{}/{}/manifests/{}", self.url, image_name, oci_manifest),
            self.token.as_ref(),
            headers,
        )
        .await?;

        let res = res
            .json::<ManifestV2Response>()
            .await
            .map_err(|e| anyhow!(e).context("Failed to parse response"))?;

        trace!("Manifest : {:?}", res);

        Ok(res)
    }

    async fn get_layer(&self, image_name: &str, layer_metadata: &LayerMetadata) -> Result<Layer> {
        // Make a request to the docker hub to get the layer from his blobSum
        let res = run_get_request(
            &format!(
                "{}/{}/blobs/{}",
                self.url, image_name, layer_metadata.blob_sum
            ),
            self.token.as_ref(),
            Vec::new(),
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
        image_name: &str,
        layers_metadata: Vec<LayerMetadata>,
    ) -> Result<Vec<Layer>> {
        let mut layers = Vec::new();
        for layer_metadata in layers_metadata.iter() {
            info!("Pulling layer {:?}", layer_metadata.blob_sum);

            let layer = self
                .get_layer(image_name, layer_metadata)
                .await
                .map_err(|e| anyhow!(e).context("Failed to pull layer"))?;
            layers.push(layer);

            debug!("Successfully pulled layer : {:?}", layer_metadata.blob_sum);
        }

        debug!("Successfully pulled all layers");

        Ok(layers)
    }

    pub async fn get_image(&mut self, image_full_name: &str) -> Result<Image> {
        let mut image = Image::new(image_full_name)?;
        let image_name = &image.name();
        self.set_token(image_name).await?;
        let layers_metadata = self.get_layers_metadata(image_name, image.tag()).await?;
        let layers = self.get_layers(image_name, layers_metadata).await?;

        image.layers = layers;

        Ok(image)
    }
}
