use rusqlite::{Connection, Result};

#[derive(Debug, Clone)]
pub struct Verse {
    pub chapter: u32,
    pub verse: u32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub book: String,
    pub chapter: u32,
    pub verse: u32,
    pub text: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn get_versions(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT bible FROM verses ORDER BY bible")?;
        let versions = stmt.query_map([], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(versions)
    }

    pub fn get_books(&self, version: &str) -> Result<Vec<String>> {
        // Since we order by occurrence, actually we should order canonically.
        // We can just rely on the canonical order that was inserted if we didn't store an ID,
        // but DISTINCT will mess up insertion order.
        // It's better to hardcode the canonical order or just query distinct and sort it ourselves.
        // Wait, since we are doing TUI, let's just query distinct and use our own canonical list for sorting.
        
        let mut stmt = self.conn.prepare("SELECT DISTINCT book FROM verses WHERE bible = ?")?;
        let books = stmt.query_map([version], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect::<Vec<String>>();
        
        let canonical_order = vec![
            "Genesis", "Exodus", "Leviticus", "Numbers", "Deuteronomy",
            "Joshua", "Judges", "Ruth", "1 Samuel", "2 Samuel",
            "1 Kings", "2 Kings", "1 Chronicles", "2 Chronicles", "Ezra",
            "Nehemiah", "Esther", "Job", "Psalms", "Proverbs",
            "Ecclesiastes", "Song of Solomon", "Isaiah", "Jeremiah", "Lamentations",
            "Ezekiel", "Daniel", "Hosea", "Joel", "Amos",
            "Obadiah", "Jonah", "Micah", "Nahum", "Habakkuk",
            "Zephaniah", "Haggai", "Zechariah", "Malachi",
            "Matthew", "Mark", "Luke", "John", "Acts",
            "Romans", "1 Corinthians", "2 Corinthians", "Galatians", "Ephesians",
            "Philippians", "Colossians", "1 Thessalonians", "2 Thessalonians",
            "1 Timothy", "2 Timothy", "Titus", "Philemon", "Hebrews",
            "James", "1 Peter", "2 Peter", "1 John", "2 John",
            "3 John", "Jude", "Revelation"
        ];
        
        let mut sorted_books = books;
        sorted_books.sort_by_key(|b| canonical_order.iter().position(|&r| r == b).unwrap_or(100));
        Ok(sorted_books)
    }

    pub fn get_chapters(&self, version: &str, book: &str) -> Result<Vec<u32>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT chapter FROM verses WHERE bible = ? AND book = ? ORDER BY chapter")?;
        let chapters = stmt.query_map([version, book], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(chapters)
    }

    pub fn get_chapter(&self, version: &str, book: &str, chapter: u32) -> Result<Vec<Verse>> {
        let mut stmt = self.conn.prepare("SELECT verse, text FROM verses WHERE bible = ? AND book = ? AND chapter = ? ORDER BY verse")?;
        let verses = stmt.query_map((version, book, chapter), |row| {
            Ok(Verse {
                chapter,
                verse: row.get(0)?,
                text: row.get(1)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
        Ok(verses)
    }

    pub fn search(&self, version: &str, query: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare("SELECT book, chapter, verse, text FROM verses WHERE bible = ?1")?;
        
        let mut results = Vec::new();
        
        // Try to compile the query as a regex
        let re = regex::RegexBuilder::new(query).case_insensitive(true).build();
        
        let rows = stmt.query_map([version], |row| {
            Ok(SearchResult {
                book: row.get(0)?,
                chapter: row.get(1)?,
                verse: row.get(2)?,
                text: row.get(3)?,
            })
        })?;

        if let Ok(regex) = re {
            for row in rows {
                if let Ok(res) = row {
                    if regex.is_match(&res.text) {
                        results.push(res);
                    }
                }
            }
        } else {
            // Fallback to basic lowercase text search if regex is invalid (e.g. while typing '[')
            let q_lower = query.to_lowercase();
            for row in rows {
                if let Ok(res) = row {
                    if res.text.to_lowercase().contains(&q_lower) {
                        results.push(res);
                    }
                }
            }
        }
        
        Ok(results)
    }
}
