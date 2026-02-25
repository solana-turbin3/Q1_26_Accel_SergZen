pub mod serializer;
pub mod storage;
pub mod person;

#[cfg(test)]
mod tests {
    use crate::{
        person::Person,
        serializer::{
            borsh::BorshSerializer,
            serde::JsonSerializer,
            wincode::WincodeSerializer,
        },
        storage::Storage,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn make_person() -> Person {
        Person {
            name: "Alice".to_string(),
            age: 30,
        }
    }

    #[test]
    fn borsh_roundtrip() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let loaded = storage.load()?;
        assert_eq!(person, loaded);
        Ok(())
    }

    #[test]
    fn borsh_name_preserved() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.name, "Alice");
        Ok(())
    }

    #[test]
    fn borsh_age_preserved() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.age, 30);
        Ok(())
    }

    #[test]
    fn borsh_empty_name() -> TestResult {
        let person = Person { name: "".to_string(), age: 0 };
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn borsh_unicode_name() -> TestResult {
        let person = Person { name: "Alice 🦀".to_string(), age: 99 };
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn borsh_max_age() -> TestResult {
        let person = Person { name: "Bob".to_string(), age: u32::MAX };
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.age, u32::MAX);
        Ok(())
    }

    #[test]
    fn borsh_has_data_after_save() -> TestResult {
        let mut storage = Storage::new(BorshSerializer);
        assert!(!storage.has_data());
        storage.save(&make_person())?;
        assert!(storage.has_data());
        Ok(())
    }

    #[test]
    fn json_roundtrip() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person)?;
        let loaded = storage.load()?;
        assert_eq!(person, loaded);
        Ok(())
    }

    #[test]
    fn json_name_preserved() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.name, "Alice");
        Ok(())
    }

    #[test]
    fn json_age_preserved() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.age, 30);
        Ok(())
    }

    #[test]
    fn json_empty_name() -> TestResult {
        let person = Person { name: "".to_string(), age: 0 };
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn json_unicode_name() -> TestResult {
        let person = Person { name: "Alice 🦀".to_string(), age: 99 };
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn json_max_age() -> TestResult {
        let person = Person { name: "Bob".to_string(), age: u32::MAX };
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.age, u32::MAX);
        Ok(())
    }

    #[test]
    fn json_has_data_after_save() -> TestResult {
        let mut storage = Storage::new(JsonSerializer);
        assert!(!storage.has_data());
        storage.save(&make_person())?;
        assert!(storage.has_data());
        Ok(())
    }

    #[test]
    fn wincode_roundtrip() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person)?;
        let loaded = storage.load()?;
        assert_eq!(person, loaded);
        Ok(())
    }

    #[test]
    fn wincode_name_preserved() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.name, "Alice");
        Ok(())
    }

    #[test]
    fn wincode_age_preserved() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.age, 30);
        Ok(())
    }

    #[test]
    fn wincode_empty_name() -> TestResult {
        let person = Person { name: "".to_string(), age: 0 };
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn wincode_unicode_name() -> TestResult {
        let person = Person { name: "Alice 🦀".to_string(), age: 99 };
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn wincode_max_age() -> TestResult {
        let person = Person { name: "Bob".to_string(), age: u32::MAX };
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person)?;
        let loaded: Person = storage.load()?;
        assert_eq!(loaded.age, u32::MAX);
        Ok(())
    }

    #[test]
    fn wincode_has_data_after_save() -> TestResult {
        let mut storage = Storage::new(WincodeSerializer);
        assert!(!storage.has_data());
        storage.save(&make_person())?;
        assert!(storage.has_data());
        Ok(())
    }

    #[test]
    fn borsh_load_without_save_returns_error() {
        let storage: Storage<Person, _> = Storage::new(BorshSerializer);
        assert!(storage.load().is_err());
    }

    #[test]
    fn json_load_without_save_returns_error() {
        let storage: Storage<Person, _> = Storage::new(JsonSerializer);
        assert!(storage.load().is_err());
    }

    #[test]
    fn wincode_load_without_save_returns_error() {
        let storage: Storage<Person, _> = Storage::new(WincodeSerializer);
        assert!(storage.load().is_err());
    }

    #[test]
    fn convert_borsh_to_json() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let converted = storage.convert(JsonSerializer)?;
        let loaded: Person = converted.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn convert_borsh_to_wincode() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let converted = storage.convert(WincodeSerializer)?;
        let loaded: Person = converted.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn convert_json_to_borsh() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person)?;
        let converted = storage.convert(BorshSerializer)?;
        let loaded: Person = converted.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn convert_json_to_wincode() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(JsonSerializer);
        storage.save(&person)?;
        let converted = storage.convert(WincodeSerializer)?;
        let loaded: Person = converted.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn convert_wincode_to_borsh() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person)?;
        let converted = storage.convert(BorshSerializer)?;
        let loaded: Person = converted.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn convert_wincode_to_json() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(WincodeSerializer);
        storage.save(&person)?;
        let converted = storage.convert(JsonSerializer)?;
        let loaded: Person = converted.load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn convert_preserves_has_data() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let converted = storage.convert(JsonSerializer)?;
        assert!(converted.has_data());
        Ok(())
    }

    #[test]
    fn convert_without_save_returns_error() {
        let storage: Storage<Person, _> = Storage::new(BorshSerializer);
        assert!(storage.convert(JsonSerializer).is_err());
    }

    #[test]
    fn convert_chain_borsh_json_wincode() -> TestResult {
        let person = make_person();
        let mut storage = Storage::new(BorshSerializer);
        storage.save(&person)?;
        let loaded: Person = storage
            .convert(JsonSerializer)?
            .convert(WincodeSerializer)?
            .load()?;
        assert_eq!(loaded, person);
        Ok(())
    }

    #[test]
    fn all_serializers_same_result() -> TestResult {
        let person = make_person();

        let mut s1 = Storage::new(BorshSerializer);
        s1.save(&person)?;
        let r1: Person = s1.load()?;

        let mut s2 = Storage::new(JsonSerializer);
        s2.save(&person)?;
        let r2: Person = s2.load()?;

        let mut s3 = Storage::new(WincodeSerializer);
        s3.save(&person)?;
        let r3: Person = s3.load()?;

        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
        Ok(())
    }
}
