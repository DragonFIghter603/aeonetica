use aeonetica_engine::{util::vector::Vector2, log};

use super::*;

pub struct TextArea {
    position: Vector2<f32>,
    z_index: u8,

    max_len: u32,
    content: String,
    
    shader: shader::Program,
    font: Rc<BitmapFont>,
    font_size: f32,
    spacing: f32,

    location: Option<VertexLocation>,
    vertices: Vec<VertexTuple3<[f32; 3], [f32; 2], Sampler2D>>,
    indices: Vec<u32>
}

impl Renderable for TextArea {
    fn location(&self) -> &Option<VertexLocation> {
        &self.location
    }

    fn set_location(&mut self, location: VertexLocation) {
        self.location = Some(location)
    }

    fn texture_id(&self) -> Option<super::RenderID> {
        Some(self.font.sprite_sheet().texture().id())
    }

    fn vertex_data<'a>(&'a mut self) -> VertexData<'a> {
        if self.vertices.is_empty() {
            self.vertices.clear();
            self.recalculate_vertex_data();
        }

        log!("{:#?}", self.vertices);

        VertexData::new_textured(
            unsafe { std::mem::transmute::<_, &mut [u8]>(self.vertices.as_mut_slice()) }, 
            self.indices.as_slice(), 
            TEXT_AREA_LAYOUT.with(|l| l.clone()),
            self.shader,
            self.z_index,
            self.font.sprite_sheet().texture().id()
        )
    }
}

thread_local! {
    static TEXT_AREA_LAYOUT: Rc<BufferLayout> = Rc::new(TextArea::Vertices::build());
}

impl TextArea {
    type Vertices = BufferLayoutBuilder<(Vertex, TexCoord, TextureID)>;

    fn gen_indices(num_chars: usize) -> Vec<u32> {
        let mut indices = Vec::with_capacity(num_chars * 6);
        for i in 0 .. num_chars {
            let i = i as u32 * 4;
            indices.extend_from_slice(&[i, i + 1, i + 2, i + 2, i + 3, i])
        }
        indices
    }

    pub fn new(position: Vector2<f32>, z_index: u8, font_size: f32, spacing: f32, max_len: u32, shader: shader::Program, font: Rc<BitmapFont>) -> Self {
        Self {
            position,
            z_index,
            max_len,
            content: String::with_capacity(max_len as usize),
            shader,
            font,
            font_size,
            spacing,
            location: None,
            vertices: Vec::with_capacity(4 * max_len as usize),
            indices: Self::gen_indices(max_len as usize)
        }
    }

    pub fn from_string(position: Vector2<f32>, z_index: u8, font_size: f32, spacing: f32, shader: shader::Program, font: Rc<BitmapFont>, content: String) -> Self {
        let len = content.len();
        Self {
            position,
            z_index,
            max_len: len as u32,
            content,
            shader,
            font,
            font_size,
            spacing,
            location: None,
            vertices: Vec::with_capacity(4 * len),
            indices: Self::gen_indices(len)
        }
    }

    pub fn recalculate_vertex_data(&mut self) {
        let size = self.font_size / self.font.char_size().y;
        let half_size = self.font.char_size() * [size, size].into() / Vector2::new(2.0, 2.0);

        let position = self.position;

        let mut x_offset = 0.0;

        for c in self.content.chars() {
            let position = Vector2::new(x_offset, position.y());

            let char_idx = self.font.char_index(c);
            if char_idx.is_none() {
                continue;
            }
            let char_idx = *char_idx.unwrap();

            let width = self.font.index_width(char_idx) as f32;
            x_offset += width * self.font_size + self.spacing;

            let char_sprite = self.font.sprite_sheet().get(char_idx);
            if char_sprite.is_none() {
                continue;
            }
            let char_sprite = char_sprite.unwrap();

            self.vertices.extend_from_slice(&Self::Vertices::array([
                vertex!([position.x() - half_size.x(), position.y() - half_size.y(), 0.0], [char_sprite.left(),  char_sprite.top()   ], Sampler2D(0)),
                vertex!([position.x() + half_size.x(), position.y() - half_size.y(), 0.0], [char_sprite.right(), char_sprite.top()   ], Sampler2D(0)),
                vertex!([position.x() + half_size.x(), position.y() + half_size.y(), 0.0], [char_sprite.right(), char_sprite.bottom()], Sampler2D(0)),
                vertex!([position.x() - half_size.x(), position.y() + half_size.y(), 0.0], [char_sprite.left(),  char_sprite.bottom()], Sampler2D(0))
            ]));
        }
    }
}