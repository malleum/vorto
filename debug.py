import re

texts = [
    r'\+w Go|strong="G4198"\+w* \+w into|strong="G1519"\+w* \+w all|strong="G3956"\+w*',
    r'[\w is|strong="123"\w*]',
    r'The \nd Lord\nd* says \q1 this is poetry \q2 and more \b',
    r'\w word \w*', 
    r'\f + \fr 1:1 \ft footnote with \+w nested|strong="1"\+w* \f*',
    r'\v 18 \w Jesus|strong="G2424"\w* \w came|strong="G4334"\w* \w to|strong="G2532"\w* \w them|strong="G3588"\w* \w and|strong="G2532"\w* \w spoke|strong="G2980"\w* \w to|strong="G2532"\w* \w them|strong="G3588"\w*, \w saying|strong="G3004"\w*, \wj “\+w All|strong="G3956"\+w* \+w authority|strong="G1849"\+w* \+w has|strong="G2532"\+w* \+w been|strong="G2532"\+w* \+w given|strong="G1325"\+w* \+w to|strong="G2532"\+w* \+w me|strong="G1325"\+w* \+w in|strong="G1722"\+w* \+w heaven|strong="G3772"\+w* \+w and|strong="G2532"\+w* \+w on|strong="G1909"\+w* \+w earth|strong="G1093"\+w*. \wj*'
]

def strip_usfm_tags(text):
    # Footnotes and cross references
    text = re.sub(r'\\[fx]\s.*?\\[fx]\*', '', text)
    
    # 1. Tags with attributes (strictly no inner tags)
    while True:
        prev = text
        text = re.sub(r'(\\[+a-z0-9]+)\s+([^|\\]+?)\|[^\\\*]*?\1\*', r'\2', text)
        if prev == text:
            break
            
    # 2. Tags without attributes (strictly no inner tags)
    while True:
        prev = text
        text = re.sub(r'(\\[+a-z0-9]+)\s+([^|\\]*?)\1\*', r'\2', text)
        if prev == text:
            break
            
    # 3. Any remaining structural tags or wrapper tags (like \wj)
    text = re.sub(r'\\[+a-z0-9]+\*', '', text) # strip closing tags
    text = re.sub(r'\\[+a-z0-9]+\s*', '', text) # strip opening tags
    
    text = re.sub(r'\[\s*([^\]]+?)\s*\]', r'\1', text)
    text = re.sub(r'\s+', ' ', text).strip()
    return text

for t in texts:
    print(strip_usfm_tags(t))
