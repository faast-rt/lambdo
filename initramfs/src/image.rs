use std::{
    collections::HashMap,
    fs::File,
    io::{Cursor, Read, Write},
};

use anyhow::{anyhow, Result};
use bytes::Bytes;
use cpio::{newc::Builder, write_cpio};
use libflate::gzip::{Decoder, Encoder};
use log::{debug, info};
use serde::Deserialize;
use tar::Archive;

/// Trait to abstract File reading
pub trait FileHandler: Write + Read + Sized {
    fn create(path: &str) -> Result<Self>;
    fn open(path: &str) -> Result<Self>;
}

impl FileHandler for File {
    fn create(path: &str) -> Result<Self> {
        File::create(path).map_err(|e| anyhow!(e).context("Failed to create file"))
    }

    fn open(path: &str) -> Result<Self> {
        File::open(path).map_err(|e| anyhow!(e).context("Failed to open file"))
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LayerMetadata {
    pub blob_sum: String,
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

        if name.is_empty() {
            return Err(anyhow!("Please provide and image name and not just a tag"));
        }

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

    pub fn export_to_initramfs<Handler: FileHandler>(
        &self,
        init_path: &str,
        agent_path: &str,
        agent_config_path: &str,
    ) -> Result<Handler> {
        // Write the cpio to disk
        let file_name = format!("initramfs-{}-{}.img", self.name.replace('/', "-"), self.tag);
        let archive = Encoder::new(
            Handler::create(&file_name).map_err(|e| anyhow!(e).context("Failed to create file"))?,
        )
        .map_err(|e| anyhow!(e).context("Failed to create gzip encoder"))?;

        let mut entries: HashMap<String, (Builder, Cursor<Vec<u8>>)> = HashMap::new();

        for layer in self.layers.clone().into_iter() {
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

                // This means we need to delete everything that is in the parent directory
                if path.contains(".wh..wh..opq") {
                    debug!("Found opaque whiteout file : {}", &path);

                    let parent = path.trim_end_matches(".wh..wh..opq");
                    let keys = entries
                        .keys()
                        .filter(|key| key.starts_with(parent))
                        .cloned()
                        .collect::<Vec<_>>();

                    keys.iter().for_each(|key| {
                        entries.remove(key);
                    });

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
                if headers.entry_type().is_symlink() {
                    let link_path = headers
                        .link_name()
                        .map_err(|e| anyhow!(e).context("Failed to get link name of entry"))?
                        .ok_or(anyhow!("Failed to get link name of entry"))?
                        .to_str()
                        .ok_or_else(|| anyhow!("Failed to convert link name to string"))?
                        .to_string();

                    contents.extend_from_slice(link_path.as_bytes());
                } else {
                    entry
                        .read_to_end(&mut contents)
                        .map_err(|e| anyhow!(e).context("Failed to read entry"))?;
                }

                entries.insert(path, (builder, Cursor::new(contents)));
            }
        }

        let mut init_file =
            Handler::open(init_path).map_err(|e| anyhow!(e).context("Failed to open init file"))?;
        let mut agent_file = Handler::open(agent_path)
            .map_err(|e| anyhow!(e).context("Failed to open init file"))?;
        let mut agent_config_file = Handler::open(agent_config_path)
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

        info!("Writing cpio to disk");

        let inputs = entries.drain().map(|(_, data)| data);

        let archive =
            write_cpio(inputs, archive).map_err(|e| anyhow!(e).context("Failed to write cpio"))?;

        let handler = archive
            .finish()
            .into_result()
            .map_err(|e| anyhow!(e).context("Failed to finish writing cpio"))?;

        debug!("Successfully wrote cpio to disk");

        Ok(handler)
    }
}

#[cfg(test)]
mod test {
    use super::{FileHandler, Image};
    use anyhow::Ok;
    use std::env;
    use std::io::{Read, Write};

    const VALID_IMAGE_NAME: &str = "my_awesome_img:14md35";
    const VALID_IMAGE_NAME_FROM_HUB: &str = "bitnami/mongodb:latest";

    #[derive(Debug, Clone)]
    struct MockFileHandler {
        vec: Vec<u8>,
    }

    impl Read for MockFileHandler {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let len = std::cmp::min(buf.len(), self.vec.len());
            buf[..len].copy_from_slice(&self.vec[..len]);
            self.vec = self.vec[len..].to_vec();
            std::result::Result::Ok(len)
        }
    }

    impl Write for MockFileHandler {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.vec.extend_from_slice(buf);
            std::result::Result::Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            std::result::Result::Ok(())
        }
    }

    impl FileHandler for MockFileHandler {
        fn create(path: &str) -> anyhow::Result<Self>
        where
            Self: std::marker::Sized,
        {
            Ok(Self::new(path))
        }

        fn open(path: &str) -> anyhow::Result<Self>
        where
            Self: std::marker::Sized,
        {
            Ok(Self::new(path))
        }
    }

    impl MockFileHandler {
        fn new(_path: &str) -> Self {
            Self { vec: Vec::new() }
        }
    }

    #[test]
    pub fn valid_image_name() {
        let image1 = Image::new(VALID_IMAGE_NAME);
        assert!(image1.is_ok());

        let image1 = image1.unwrap();
        assert_eq!(image1.name(), "library/my_awesome_img");
        assert_eq!(image1.tag(), "14md35");

        let image2 = Image::new(VALID_IMAGE_NAME_FROM_HUB);
        assert!(image2.is_ok());

        let image2 = image2.unwrap();
        assert_eq!(image2.name(), "bitnami/mongodb");
        assert_eq!(image2.tag(), "latest");
    }

    #[test]
    pub fn invalid_image_name() {
        let image = Image::new(":tag_but_with_no_image");
        assert!(image.is_err());
    }

    #[test]
    pub fn test_initramfs_export() {
        let image = Image::new(VALID_IMAGE_NAME);

        let image_filename = format!("{}/init", env::temp_dir().display());
        let agent_filename = format!("{}/agent", env::temp_dir().display());
        let agent_config_filename = format!("{}/agent_config", env::temp_dir().display());

        MockFileHandler::new(&image_filename);
        MockFileHandler::new(&agent_filename);
        MockFileHandler::new(&agent_config_filename);

        // checks
        let handler = image.unwrap().export_to_initramfs::<MockFileHandler>(
            image_filename.as_str(),
            agent_filename.as_str(),
            agent_config_filename.as_str(),
        );

        let mut handler = handler.unwrap();

        let mut read_buf = [0; 2];
        let _ = handler.read(&mut read_buf).unwrap();

        assert_eq!(read_buf, [0x1F, 0x8b]);
    }
}
