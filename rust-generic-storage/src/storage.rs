use crate::serializer::Serializer;

pub struct Storage<T, S> {
    data: Option<Vec<u8>>,
    serializer: S,
    _marker: std::marker::PhantomData<T>,
}

impl<T, S: Serializer<T>> Storage<T, S> {
    pub fn new(serializer: S) -> Self {
        Self {
            data: None,
            serializer,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn save(&mut self, value: &T) -> Result<(), Box<dyn std::error::Error>> {
        self.data = Some(self.serializer.to_bytes(value)?);
        Ok(())
    }

    pub fn load(&self) -> Result<T, Box<dyn std::error::Error>> {
        if let Some(ref bytes) = self.data {
            self.serializer.from_bytes(bytes)
        } else {
            Err("No data to load".into())
        }
    }

    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    pub fn convert<S2: Serializer<T>>(self, new_serializer: S2) -> Result<Storage<T, S2>, Box<dyn std::error::Error>> {
        let bytes = self.data.ok_or("No data to convert")?;
        let value = self.serializer.from_bytes(&bytes)?;
        let new_bytes = new_serializer.to_bytes(&value)?;

        Ok(Storage {
            data: Some(new_bytes),
            serializer: new_serializer,
            _marker: std::marker::PhantomData,
        })
    }
}