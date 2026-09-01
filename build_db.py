import os
import re
import sqlite3
import zipfile
import sys
from pathlib import Path

book_map = {
    'GEN': 'Genesis', 'EXO': 'Exodus', 'LEV': 'Leviticus', 'NUM': 'Numbers', 'DEU': 'Deuteronomy',
    'JOS': 'Joshua', 'JDG': 'Judges', 'RUT': 'Ruth', '1SA': '1 Samuel', '2SA': '2 Samuel',
    '1KI': '1 Kings', '2KI': '2 Kings', '1CH': '1 Chronicles', '2CH': '2 Chronicles', 'EZR': 'Ezra',
    'NEH': 'Nehemiah', 'EST': 'Esther', 'JOB': 'Job', 'PSA': 'Psalms', 'PRO': 'Proverbs',
    'ECC': 'Ecclesiastes', 'SNG': 'Song of Solomon', 'ISA': 'Isaiah', 'JER': 'Jeremiah', 'LAM': 'Lamentations',
    'EZK': 'Ezekiel', 'DAN': 'Daniel', 'HOS': 'Hosea', 'JOL': 'Joel', 'AMO': 'Amos',
    'OBA': 'Obadiah', 'JON': 'Jonah', 'MIC': 'Micah', 'NAM': 'Nahum', 'HAB': 'Habakkuk',
    'ZEP': 'Zephaniah', 'HAG': 'Haggai', 'ZEC': 'Zechariah', 'MAL': 'Malachi',
    'MAT': 'Matthew', 'MRK': 'Mark', 'LUK': 'Luke', 'JHN': 'John', 'ACT': 'Acts',
    'ROM': 'Romans', '1CO': '1 Corinthians', '2CO': '2 Corinthians', 'GAL': 'Galatians', 'EPH': 'Ephesians',
    'PHP': 'Philippians', 'COL': 'Colossians', '1TH': '1 Thessalonians', '2TH': '2 Thessalonians',
    '1TI': '1 Timothy', '2TI': '2 Timothy', 'TIT': 'Titus', 'PHM': 'Philemon', 'HEB': 'Hebrews',
    'JAS': 'James', '1PE': '1 Peter', '2PE': '2 Peter', '1JN': '1 John', '2JN': '2 John',
    '3JN': '3 John', 'JUD': 'Jude', 'REV': 'Revelation'
}

def strip_usfm_tags(text):
    text = re.sub(r'\\w\s+([^|]+?)\|[^\]]*?\\w\*', r'\1', text)
    text = re.sub(r'\\f.*?\\f\*', '', text)
    text = re.sub(r'\\x.*?\\x\*', '', text)
    text = re.sub(r'\\[a-z]+\s+(.*?)\\[a-z]+\*', r'\1', text)
    text = re.sub(r'\\[a-z0-9]+\s?', '', text)
    text = re.sub(r'\[\\w\s+is\|[^\]]*?\\w\*\]', 'is', text)
    text = re.sub(r'\[\\w\s+([^|]+?)\|[^\]]*?\\w\*\]', r'\1', text)
    text = re.sub(r'\[\\w\s+([^\]]+)\]', r'\1', text)
    text = re.sub(r'\[\s*\]', '', text)
    text = re.sub(r'\s+', ' ', text).strip()
    return text

def build_db(db_path, zips):
    conn = sqlite3.connect(db_path)
    c = conn.cursor()
    c.execute('''CREATE TABLE verses (bible TEXT, book TEXT, chapter INTEGER, verse INTEGER, text TEXT)''')
                 
    for version, zip_file in zips.items():
        print(f"Processing {version}...")
        with zipfile.ZipFile(zip_file, 'r') as z:
            for filename in z.namelist():
                if not filename.endswith('.usfm'):
                    continue
                content = z.read(filename).decode('utf-8', errors='ignore')
                
                book_code = None
                for line in content.splitlines():
                    if line.startswith(r'\id '):
                        parts = line.strip().split()
                        if len(parts) > 1:
                            book_code = parts[1]
                        break
                if book_code not in book_map:
                    continue
                book_name = book_map[book_code]
                
                current_chapter, current_verse = 0, 0
                verse_text = []
                
                def flush_verse():
                    if current_chapter > 0 and current_verse > 0 and verse_text:
                        text = strip_usfm_tags(' '.join(verse_text))
                        if text:
                            c.execute("INSERT INTO verses (bible, book, chapter, verse, text) VALUES (?, ?, ?, ?, ?)",
                                      (version, book_name, current_chapter, current_verse, text))
                        verse_text.clear()

                for line in content.splitlines():
                    line = line.strip()
                    if line.startswith(r'\c '):
                        flush_verse()
                        parts = line.split()
                        if len(parts) > 1 and parts[1].isdigit():
                            current_chapter = int(parts[1])
                            current_verse = 0
                    elif line.startswith(r'\v '):
                        flush_verse()
                        v_match = re.search(r'\\v\s+(\d+)', line)
                        if v_match:
                            current_verse = int(v_match.group(1))
                        else:
                            continue
                        text_part = re.sub(r'^\\v\s+[0-9-]+\s*', '', line)
                        if text_part:
                            verse_text.append(text_part)
                    else:
                        if current_chapter > 0 and current_verse > 0:
                            if line.startswith(r'\s') or line.startswith(r'\h') or line.startswith(r'\mt') or line.startswith(r'\toc') or line.startswith(r'\id '):
                                pass
                            else:
                                verse_text.append(line)
                flush_verse()
    
    c.execute("CREATE INDEX idx_verses_lookup ON verses (bible, book, chapter)")
    c.execute("CREATE VIRTUAL TABLE verses_fts USING fts5(bible, book, chapter, verse, text)")
    c.execute("INSERT INTO verses_fts SELECT bible, book, chapter, verse, text FROM verses")
    conn.commit()
    conn.close()

if __name__ == '__main__':
    db_path = sys.argv[1]
    zips = {'BSB': sys.argv[2], 'WEB': sys.argv[3], 'LSV': sys.argv[4], 'Esperanto': sys.argv[5], 'Vulgate': sys.argv[6]}
    build_db(db_path, zips)
