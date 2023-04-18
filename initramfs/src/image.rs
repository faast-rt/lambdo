use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read},
};

use anyhow::{anyhow, Result};
use bytes::Bytes;
use cpio::{newc::Builder, write_cpio};
use libflate::gzip::{Decoder, Encoder};
use log::debug;
use serde::Deserialize;
use tar::Archive;

#[derive(Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct LayerMetadata {
    pub blobSum: String,
}

#[derive(Clone)]
pub struct Layer {
    pub metadata: LayerMetadata,
    pub content: Cursor<Bytes>,
}

pub struct Image {
    name: String,
    tag: String,
    pub layers: Vec<Layer>,
}

impl Image {
    pub fn new(full_name: &str) -> Result<Self> {
        // Extract the registry and tag from the image
        let mut parts = full_name.split(':');
        let name = parts.next().ok_or_else(|| anyhow!("Invalid image name"))?;
        let tag = parts.next().unwrap_or("latest");

        // If the registry doesn't contain a '/', it's in "library/", meaning it's from the docker hub official images
        let image = Self {
            name: if !name.contains('/') {
                format!("library/{}", name)
            } else {
                name.to_string()
            },
            tag: tag.to_string(),
            layers: Vec::new(),
        };

        debug!("Successfully parsed image : {:?}", image.name());

        Ok(image)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn tag(&self) -> &str {
        &self.tag
    }

    pub fn export_to_initramfs(
        &self,
        init_path: &str,
        agent_path: &str,
        agent_config_path: &str,
    ) -> Result<()> {
        // Write the cpio to disk
        let file_name = format!("initramfs-{}-{}.img", self.name.replace('/', "-"), self.tag);
        let archive = Encoder::new(
            File::create(file_name).map_err(|e| anyhow!(e).context("Failed to create file"))?,
        )
        .map_err(|e| anyhow!(e).context("Failed to create gzip encoder"))?;

        let mut entries = HashMap::new();

        for layer in self.layers.clone() {
            let mut archive = Archive::new(Decoder::new(layer.content)?);

            for entry in archive
                .entries()
                .map_err(|e| anyhow!(e).context("Failed to get archive entries"))?
            {
                let mut entry = entry.map_err(|e| anyhow!(e).context("Failed to get entry"))?;
                let headers = entry.header();
                let path = headers
                    .path()
                    .map_err(|e| anyhow!(e).context("Failed to get path of entry"))?
                    .to_str()
                    .ok_or_else(|| anyhow!("Failed to convert path to string"))?
                    .to_string();

                if entries.contains_key(&path) {
                    continue;
                }

                let mode = headers
                    .mode()
                    .map_err(|e| anyhow!(e).context("Failed to get mode of entry"))?;

                let builder = Builder::new(&path)
                    .uid(
                        headers
                            .uid()
                            .map_err(|e| anyhow!(e).context("Failed to get uid of entry"))?
                            as u32,
                    )
                    .gid(
                        headers
                            .gid()
                            .map_err(|e| anyhow!(e).context("Failed to get gid of entry"))?
                            as u32,
                    )
                    .mode(mode);

                debug!("Adding {} to archive", &path);

                let mut contents = Vec::new();
                entry
                    .read_to_end(&mut contents)
                    .map_err(|e| anyhow!(e).context("Failed to read entry"))?;

                entries.insert(path, (builder, Cursor::new(contents)));
            }
        }

        let mut init_file =
            File::open(init_path).map_err(|e| anyhow!(e).context("Failed to open init file"))?;
        let mut agent_file =
            File::open(agent_path).map_err(|e| anyhow!(e).context("Failed to open init file"))?;
        let mut agent_config_file = File::open(agent_config_path)
            .map_err(|e| anyhow!(e).context("Failed to open agent config file"))?;

        let mut init_content = Vec::new();
        let mut agent_content = Vec::new();
        let mut agent_config_content = Vec::new();

        init_file.read_to_end(&mut init_content)?;
        agent_file.read_to_end(&mut agent_content)?;
        agent_config_file.read_to_end(&mut agent_config_content)?;

        entries.insert(
            "init".to_string(),
            (Builder::new("init").mode(33277), Cursor::new(init_content)),
        );
        entries.insert(
            "agent".to_string(),
            (
                Builder::new("agent").mode(33277),
                Cursor::new(agent_content),
            ),
        );
        entries.insert(
            "config.yaml".to_string(),
            (
                Builder::new("config.yaml").mode(33204),
                Cursor::new(agent_config_content),
            ),
        );

        let test = entries
            .drain()
            .map(|(_, (builder, contents))| (builder, contents));
        let archive =
            write_cpio(test, archive).map_err(|e| anyhow!(e).context("Failed to write cpio"))?;
        archive
            .finish()
            .into_result()
            .map_err(|e| anyhow!(e).context("Failed to finish writing cpio"))?;

        debug!("Successfully wrote cpio to disk");

        Ok(())
    }
}
