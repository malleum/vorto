import re

texts = [
    r'\+w Go|strong="G4198"\+w* \+w into|strong="G1519"\+w* \+w all|strong="G3956"\+w*',
    r'[\w is|strong="123"\w*]',
    r'The \nd Lord\nd* says \q1 this is poetry \q2 and more \b',
    r'\w word \w*', # Wait, no attributes here, handled by rule 3
    r'\f + \fr 1:1 \ft footnote with \+w nested|strong="1"\+w* \f*', # nested tag in footnote
]

def strip_usfm_tags(text):
    text = re.sub(r'\\[fx]\s.*?\\[fx]\*', '', text)
    text = re.sub(r'(\\[+a-z0-9]+)\s+([^|]*?)\|.*?\1\*', r'\2', text)
    text = re.sub(r'(\\[+a-z0-9]+)\s+(.*?)\1\*', r'\2', text)
    text = re.sub(r'\\[+a-z0-9]+\s*', '', text)
    text = re.sub(r'\[\s*([^\]]+?)\s*\]', r'\1', text)
    text = re.sub(r'\s+', ' ', text).strip()
    return text

for t in texts:
    print("ORIGINAL:", t)
    print("STRIPPED:", strip_usfm_tags(t))
    print()

