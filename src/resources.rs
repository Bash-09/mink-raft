use std::{collections::HashMap, sync::OnceLock};

use inflector::Inflector;
use serde_json::{self, Value};

pub mod block_models;

pub struct Entity {
    pub name: String,
    pub id: u32,
    pub translation_key: String,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug)]
pub struct BlockState {
    pub name: String,
    pub id: u32,
    pub models: Option<Vec<String>>,
    pub collision_shape: Option<u64>,
}

#[derive(Debug)]
pub struct BlockTexture {
    pub index: usize,
    pub interpolation: bool,
    pub frames: Vec<image::RgbaImage>,
    pub frametime: usize,
}

pub const PLAYER_INDEX: usize = 106;

pub const MISSING_TEXTURE: BlockTexture = BlockTexture {
    index: 0,
    interpolation: false,
    frames: Vec::new(),
    frametime: 0,
};

pub fn entities() -> &'static HashMap<u32, Entity> {
    static ENTITIES: OnceLock<HashMap<u32, Entity>> = OnceLock::new();

    ENTITIES.get_or_init(|| {
        let mut entities = HashMap::new();

        let json: HashMap<String, Value> =
            serde_json::from_slice(include_bytes!("../assets/entities.min.json"))
                .expect("Failed to interpret entities.json");
        for (name, val) in json.iter() {
            if let Some(id) = val.get("id") {
                entities.insert(
                    id.as_u64().unwrap() as u32,
                    Entity {
                        name: format_name(name),
                        id: id.as_u64().unwrap() as u32,
                        translation_key: val
                            .get("loot_table")
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string(),
                        width: val.get("width").unwrap().as_f64().unwrap() as f32,
                        height: val.get("height").unwrap().as_f64().unwrap() as f32,
                    },
                );
            }
        }

        entities
    })
}

pub fn blocks() -> &'static HashMap<u32, BlockState> {
    static BLOCKS: OnceLock<HashMap<u32, BlockState>> = OnceLock::new();

    BLOCKS.get_or_init(|| {
        let mut blocks = HashMap::new();

        let json: HashMap<String, Value> =
            serde_json::from_slice(include_bytes!("../assets/blocks.min.json"))
                .expect("Failed to interpret blocks.json");
        for (name, val) in json.iter() {
            let name = format_name(name);
            for (id, state) in val.get("states").unwrap().as_object().unwrap().iter() {
                let id = id.parse().unwrap();
                blocks.insert(
                    id,
                    BlockState {
                        name: name.clone(),
                        id,
                        models: {
                            match state.get("render") {
                                // Has a single model
                                Some(serde_json::Value::Object(render)) => {
                                    if let Some(serde_json::Value::String(model)) = render.get("model") {
                                        Some(vec![model.clone()])
                                    } else {
                                        tracing::error!("Couldn't find models in block state: {:?}", state);
                                        None
                                    }
                                },
                                // Has an array of models
                                Some(serde_json::Value::Array(render)) => {
                                    let mut vec = Vec::new();

                                    for render in render {
                                        if let Some(serde_json::Value::String(model)) = render.get("model") {
                                            vec.push(model.clone());
                                        } else {
                                            if let serde_json::Value::Array(render) = render {
                                                for render in render {
                                                    if let Some(serde_json::Value::String(model)) = render.get("model") {
                                                        vec.push(model.clone());
                                                    } else {
                                                        tracing::error!("Couldn't find model in render section 2");
                                                    }
                                                }
                                            } else {
                                                println!("{:?}", render);
                                                panic!();
                                            }
                                        }
                                    }

                                    if vec.len() == 0 {
                                        println!("{:?}", render);
                                        panic!("No models in render section");
                                    }

                                    Some(vec)
                                },
                                _ => None,
                            }
                        },
                        collision_shape: {
                            match state.get("collision_shape") {
                                Some(collision_shape) => collision_shape.as_u64(),
                                None => None,
                            }
                        },
                    },
                );
            }
        }

        blocks
    })
}

/*
pub static ref BLOCK_MODELS_RAW: HashMap<String, Value> =
    serde_json::from_slice(include_bytes!("../assets/models.min.json"))
        .expect("Failed to interpret models.json");
pub static ref BLOCK_MODELS_PARSED: HashMap<String, BlockModel> = {
    let mut models = HashMap::new();

    for (key, data) in BLOCK_MODELS_RAW.iter() {
        if models.contains_key(key) { continue; }

        match BlockModel::parse(data, Some(&mut models)) {
            Ok(model) => { models.insert(key.clone(), model); },
            Err(e) => { log::debug!("Couldn't parse block model: {:?}", e); },
        }
    }

    models
};
pub static ref BLOCK_TEXTURES: HashMap<String, BlockTexture> = {
    // Get list of texture and metadata files available
    let mut textures: Vec<_> = std::fs::read_dir("assets/textures/block/")
        .expect("Couldn't find textures directory")
        .filter_map(|f| f.ok())
        .filter(|f| {
            let name = f.file_name();
            let name = name.to_string_lossy();
            name.ends_with(".png") || name.ends_with(".mcmeta")
        }).collect();
    let metadata: Vec<_> = textures.drain_filter(|f| {
        f.file_name().to_string_lossy().ends_with(".mcmeta")
    }).collect();

    let mut out = HashMap::new();

    // Load textures
    image::load(Cursor::new(&include_bytes!("../assets/missing_texture.png")), image::ImageFormat::Png).unwrap().to_rgba8();
    out.insert(String::new(), MISSING_TEXTURE);

    let mut index: usize = 1; // Reserve index 0 for missing texture
    for tex in textures {
        let full_name = tex.file_name();
        let full_name = full_name.to_string_lossy();
        let name = full_name.split(".").nth(0).unwrap();

        let data = std::fs::read(tex.path());
        if data.is_err() {continue;}
        let data = data.unwrap();
        let img = image::load(Cursor::new(&data), image::ImageFormat::Png).unwrap().to_rgba8();

        let mut frames = Vec::new();
        if img.height() == 16 {
            // Load single texture
            frames.push(img);
        } else {
            // Load as multiple textures
            let num_frames = img.height() / 16;
            for i in 0..num_frames {
                frames.push(image::SubImage::new(&img, 0, i * 16, 16, 16).to_image());
            }
        }
        let inc = frames.len();

        out.insert(format!("minecraft:block/{}", name), BlockTexture {
            index,
            interpolation: false,
            frames,
            frametime: 0,
        });

        index += inc;
    }

    // Add any metadata
    for metadata in metadata {
        let full_name = metadata.file_name();
        let full_name = full_name.to_string_lossy();
        let name = full_name.split(".").nth(0).unwrap();

        if !out.contains_key(name){continue;}
        let tex = out.get_mut(name).unwrap();

        let contents = std::fs::read_to_string(metadata.path()).unwrap();
        let meta = serde_json::from_str::<serde_json::Value>(&contents).unwrap();

        if let Some(anim) = meta.get("animation") {
            if let Some(interp) = anim.get("interpolate") {
                tex.interpolation = interp.as_bool().unwrap();
            }
            if let Some(frametime) = anim.get("frametime") {
                tex.frametime = frametime.as_u64().unwrap() as usize;
            }
        }
    }

    out
};
*/

pub fn format_name(name: &str) -> String {
    name.replace("minecraft:", "")
        .replace('_', " ")
        .to_title_case()
}
