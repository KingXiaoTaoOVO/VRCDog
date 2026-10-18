// Transliteration helper for Japanese Romaji and Hiragana conversion.
// Zero-dependency implementation for real-time message transliteration.

const KANA_TO_ROMAJI: Record<string, string> = {
  // Digraphs (Yōon)
  'きゃ': 'kya', 'きゅ': 'kyu', 'きょ': 'kyo',
  'しゃ': 'sha', 'しゅ': 'shu', 'しょ': 'sho',
  'ちゃ': 'cha', 'ちゅ': 'chu', 'ちょ': 'cho',
  'にゃ': 'nya', 'にゅ': 'nyu', 'にょ': 'nyo',
  'ひゃ': 'hya', 'ひゅ': 'hyu', 'ひょ': 'hyo',
  'みゃ': 'mya', 'みゅ': 'myu', 'みょ': 'myo',
  'りゃ': 'rya', 'りゅ': 'ryu', 'りょ': 'ryo',
  'ぎゃ': 'gya', 'ぎゅ': 'gyu', 'ぎょ': 'gyo',
  'じゃ': 'ja', 'じゅ': 'ju', 'じょ': 'jo',
  'びゃ': 'bya', 'びゅ': 'byu', 'びょ': 'byo',
  'ぴゃ': 'pya', 'ぴゅ': 'pyu', 'ぴょ': 'pyo',

  // Monophongs
  'あ': 'a', 'い': 'i', 'う': 'u', 'え': 'e', 'お': 'o',
  'か': 'ka', 'き': 'ki', 'く': 'ku', 'け': 'ke', 'こ': 'ko',
  'さ': 'sa', 'し': 'shi', 'す': 'su', 'せ': 'se', 'そ': 'so',
  'た': 'ta', 'ち': 'chi', 'つ': 'tsu', 'て': 'te', 'と': 'to',
  'な': 'na', 'に': 'ni', 'ぬ': 'nu', 'ね': 'ne', 'の': 'no',
  'は': 'ha', 'ひ': 'hi', 'ふ': 'fu', 'へ': 'he', 'ほ': 'ho',
  'ま': 'ma', 'み': 'mi', 'む': 'mu', 'め': 'me', 'mo': 'mo', 'も': 'mo',
  'や': 'ya', 'ゆ': 'yu', 'よ': 'yo',
  'ら': 'ra', 'り': 'ri', 'る': 'ru', 'れ': 're', 'ろ': 'ro',
  'わ': 'wa', 'を': 'wo', 'ん': 'n',
  'が': 'ga', 'ぎ': 'gi', 'ぐ': 'gu', 'げ': 'ge', 'ご': 'go',
  'ざ': 'za', 'じ': 'ji', 'ず': 'zu', 'ぜ': 'ze', 'ぞ': 'zo',
  'だ': 'da', 'ぢ': 'di', 'づ': 'du', 'де': 'de', 'で': 'de', 'ど': 'do',
  'ば': 'ba', 'び': 'bi', 'ぶ': 'bu', 'べ': 'be', 'ぼ': 'bo',
  'ぱ': 'pa', 'ぴ': 'pi', 'ぷ': 'pu', 'ぺ': 'pe', 'ぽ': 'po',
  'ぁ': 'a', 'ぃ': 'i', 'ぅ': 'u', 'ぇ': 'e', 'ぉ': 'o',
  'っ': '', // sokuon handled separately
  'ー': '-',
};

export function katakanaToHiragana(text: string): string {
  let result = '';
  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i);
    if (code >= 0x30a1 && code <= 0x30f6) {
      result += String.fromCharCode(code - 0x60);
    } else {
      result += text[i];
    }
  }
  return result;
}

export function toRomaji(text: string): string {
  const hira = katakanaToHiragana(text);
  let out = '';
  let i = 0;
  while (i < hira.length) {
    // Check for sokuon (small tsu)
    if (hira[i] === 'っ' && i + 1 < hira.length) {
      const nextChar = hira[i + 1];
      const nextRomaji = KANA_TO_ROMAJI[nextChar] || (i + 2 < hira.length ? KANA_TO_ROMAJI[hira.slice(i + 1, i + 3)] : '');
      if (nextRomaji && nextRomaji[0]) {
        out += nextRomaji[0];
        i++;
        continue;
      }
    }

    // Check two-character digraph
    if (i + 1 < hira.length) {
      const pair = hira.slice(i, i + 2);
      if (KANA_TO_ROMAJI[pair]) {
        out += KANA_TO_ROMAJI[pair];
        i += 2;
        continue;
      }
    }

    // Check single character
    const single = hira[i];
    if (KANA_TO_ROMAJI[single]) {
      out += KANA_TO_ROMAJI[single];
    } else {
      out += single;
    }
    i++;
  }
  return out;
}
