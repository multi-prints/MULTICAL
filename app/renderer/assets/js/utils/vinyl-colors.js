/**
 * Vinyl Color Knowledge Module
 * Comprehensive color database for sticker/vinyl printing industry
 * Based on industry standards: ORACAL 651, ORACAL 641, Reflective films, etc.
 */

const VINYL_COLOR_KNOWLEDGE = {
  // ============================================================
  // COLOR MODIFIERS/VARIANTS
  // These are suffixes/prefixes that modify base colors
  // ============================================================
  modifiers: {
    // Intensity modifiers
    dark: { adjust: -30, description: 'Darker shade' },
    light: { adjust: 30, description: 'Lighter shade' },
    medium: { adjust: 0, description: 'Standard intensity' },
    deep: { adjust: -20, description: 'Deep/rich shade' },
    pale: { adjust: 40, description: 'Pale/light shade' },
    pastel: { adjust: 50, description: 'Soft pastel shade' },
    bright: { adjust: 10, description: 'Bright/vivid shade' },
    vivid: { adjust: 15, description: 'Vivid/intense shade' },

    // Finish modifiers
    gloss: { finish: 'gloss', description: 'Glossy finish' },
    glossy: { finish: 'gloss', description: 'Glossy finish' },
    matte: { finish: 'matte', description: 'Matte/flat finish' },
    matt: { finish: 'matte', description: 'Matte/flat finish' },
    satin: { finish: 'satin', description: 'Satin finish' },
    metallic: { finish: 'metallic', description: 'Metallic finish' },
    chrome: { finish: 'chrome', description: 'Chrome/mirror finish' },
    mirror: { finish: 'mirror', description: 'Mirror finish' },
    pearl: { finish: 'pearl', description: 'Pearlescent finish' },
    fluorescent: { finish: 'fluorescent', description: 'Fluorescent/neon finish' },
    neon: { finish: 'fluorescent', description: 'Neon/fluorescent finish' },
    holographic: { finish: 'holographic', description: 'Holographic finish' },
    glitter: { finish: 'glitter', description: 'Glitter finish' },
    transparent: { finish: 'transparent', description: 'Transparent/clear' },
    translucent: { finish: 'translucent', description: 'Translucent finish' }
  },

  // ============================================================
  // BASE COLORS - ORACAL 651 STANDARD PALETTE
  // Industry standard 82 colors with gloss/matte variants
  // ============================================================
  baseColors: {
    // Whites & Neutrals
    'white': { hex: '#FFFFFF', oracalCode: '010', category: 'neutral' },
    'soft white': { hex: '#F5F5F0', oracalCode: '011', category: 'neutral' },
    'creme': { hex: '#F5F5DC', oracalCode: '012', category: 'neutral' },
    'ivory': { hex: '#FFFFF0', oracalCode: '013', category: 'neutral' },
    'beige': { hex: '#F5DEB3', oracalCode: '014', category: 'neutral' },
    'cream': { hex: '#FFFDD0', oracalCode: null, category: 'neutral' },

    // Blacks & Grays
    'black': { hex: '#1A1A1A', oracalCode: '070', category: 'neutral' },
    'grey': { hex: '#808080', oracalCode: '071', category: 'neutral' },
    'gray': { hex: '#808080', oracalCode: '071', category: 'neutral' },
    'light grey': { hex: '#D3D3D3', oracalCode: '072', category: 'neutral' },
    'light gray': { hex: '#D3D3D3', oracalCode: '072', category: 'neutral' },
    'dark grey': { hex: '#404040', oracalCode: '073', category: 'neutral' },
    'dark gray': { hex: '#404040', oracalCode: '073', category: 'neutral' },
    'charcoal': { hex: '#36454F', oracalCode: '074', category: 'neutral' },
    'slate': { hex: '#708090', oracalCode: '075', category: 'neutral' },
    'silver': { hex: '#C0C0C0', oracalCode: '076', category: 'neutral' },
    'anthracite': { hex: '#303030', oracalCode: '077', category: 'neutral' },

    // Yellows
    'yellow': { hex: '#FFD700', oracalCode: '020', category: 'yellow' },
    'lemon yellow': { hex: '#FFF44F', oracalCode: '021', category: 'yellow' },
    'bright yellow': { hex: '#FFEA00', oracalCode: '022', category: 'yellow' },
    'canary yellow': { hex: '#FFEF00', oracalCode: '023', category: 'yellow' },
    'golden yellow': { hex: '#FFDF00', oracalCode: '024', category: 'yellow' },
    'brimstone yellow': { hex: '#E6B800', oracalCode: '025', category: 'yellow' },
    'signal yellow': { hex: '#F7BA0B', oracalCode: '019', category: 'yellow' },
    'saffron': { hex: '#F4C430', oracalCode: null, category: 'yellow' },
    'amber': { hex: '#FFBF00', oracalCode: null, category: 'yellow' },
    'gold': { hex: '#FFD700', oracalCode: '024', category: 'yellow' },
    'dark yellow': { hex: '#DAA520', oracalCode: null, category: 'yellow' },

    // Oranges
    'orange': { hex: '#FF6600', oracalCode: '034', category: 'orange' },
    'bright orange': { hex: '#FF6B00', oracalCode: '034', category: 'orange' },
    'tangerine': { hex: '#FF9966', oracalCode: '034', category: 'orange' },
    'burnt orange': { hex: '#CC5500', oracalCode: '035', category: 'orange' },
    'pastel orange': { hex: '#FFB366', oracalCode: '035', category: 'orange' },
    'red orange': { hex: '#FF5349', oracalCode: '036', category: 'orange' },
    'deep orange': { hex: '#E65C00', oracalCode: null, category: 'orange' },
    'peach': { hex: '#FFCBA4', oracalCode: null, category: 'orange' },
    'apricot': { hex: '#FBCEB1', oracalCode: null, category: 'orange' },

    // Reds
    'red': { hex: '#E60012', oracalCode: '030', category: 'red' },
    'bright red': { hex: '#FF0000', oracalCode: '030', category: 'red' },
    'dark red': { hex: '#8B0000', oracalCode: '031', category: 'red' },
    'ruby red': { hex: '#9B111E', oracalCode: '032', category: 'red' },
    'cherry red': { hex: '#DE3163', oracalCode: '033', category: 'red' },
    'burgundy': { hex: '#800020', oracalCode: null, category: 'red' },
    'maroon': { hex: '#800000', oracalCode: null, category: 'red' },
    'wine': { hex: '#722F37', oracalCode: null, category: 'red' },
    'crimson': { hex: '#DC143C', oracalCode: null, category: 'red' },
    'scarlet': { hex: '#FF2400', oracalCode: null, category: 'red' },
    'brick red': { hex: '#B22222', oracalCode: null, category: 'red' },
    'signal red': { hex: '#CE1126', oracalCode: '030', category: 'red' },
    'fire red': { hex: '#FF4500', oracalCode: null, category: 'red' },
    'oxblood': { hex: '#660000', oracalCode: null, category: 'red' },

    // Pinks & Roses
    'pink': { hex: '#FF69B4', oracalCode: '041', category: 'pink' },
    'light pink': { hex: '#FFB6C1', oracalCode: '041', category: 'pink' },
    'hot pink': { hex: '#FF69B4', oracalCode: '041', category: 'pink' },
    'rose': { hex: '#FF007F', oracalCode: '042', category: 'pink' },
    'salmon': { hex: '#FA8072', oracalCode: '043', category: 'pink' },
    'coral': { hex: '#FF7F50', oracalCode: '044', category: 'pink' },
    'magenta': { hex: '#FF00FF', oracalCode: '045', category: 'pink' },
    'fuchsia': { hex: '#FF00FF', oracalCode: '045', category: 'pink' },
    'raspberry': { hex: '#E30B5C', oracalCode: null, category: 'pink' },
    'mauve': { hex: '#E0B0FF', oracalCode: null, category: 'pink' },
    'blush': { hex: '#DE5D83', oracalCode: null, category: 'pink' },

    // Purples & Violets
    'purple': { hex: '#800080', oracalCode: '050', category: 'purple' },
    'violet': { hex: '#8B5CF6', oracalCode: '051', category: 'purple' },
    'lavender': { hex: '#E6E6FA', oracalCode: '043', category: 'purple' },
    'lilac': { hex: '#C8A2C8', oracalCode: '052', category: 'purple' },
    'plum': { hex: '#DDA0DD', oracalCode: '053', category: 'purple' },
    'dark purple': { hex: '#301934', oracalCode: '054', category: 'purple' },
    'light purple': { hex: '#B19CD9', oracalCode: '055', category: 'purple' },
    'indigo': { hex: '#4B0082', oracalCode: '056', category: 'purple' },
    'amethyst': { hex: '#9966CC', oracalCode: null, category: 'purple' },
    'grape': { hex: '#6F2DA8', oracalCode: null, category: 'purple' },
    'royal purple': { hex: '#7851A9', oracalCode: null, category: 'purple' },
    'purple red': { hex: '#960018', oracalCode: '026', category: 'purple' },
    'gentian blue': { hex: '#4B0082', oracalCode: '051', category: 'purple' },

    // Blues
    'blue': { hex: '#0066CC', oracalCode: '060', category: 'blue' },
    'light blue': { hex: '#ADD8E6', oracalCode: '061', category: 'blue' },
    'dark blue': { hex: '#00008B', oracalCode: '062', category: 'blue' },
    'navy': { hex: '#000080', oracalCode: '063', category: 'blue' },
    'navy blue': { hex: '#000080', oracalCode: '063', category: 'blue' },
    'royal blue': { hex: '#4169E1', oracalCode: '064', category: 'blue' },
    'sky blue': { hex: '#87CEEB', oracalCode: '065', category: 'blue' },
    'ice blue': { hex: '#D6EAF8', oracalCode: '056', category: 'blue' },
    'baby blue': { hex: '#89CFF0', oracalCode: '061', category: 'blue' },
    'azure': { hex: '#007FFF', oracalCode: '052', category: 'blue' },
    'cobalt': { hex: '#0047AB', oracalCode: '066', category: 'blue' },
    'sapphire': { hex: '#0F52BA', oracalCode: null, category: 'blue' },
    'teal': { hex: '#008080', oracalCode: '067', category: 'blue' },
    'turquoise': { hex: '#40E0D0', oracalCode: '054', category: 'blue' },
    'cyan': { hex: '#00FFFF', oracalCode: '055', category: 'blue' },
    'aqua': { hex: '#00FFFF', oracalCode: '055', category: 'blue' },
    'king blue': { hex: '#0047AB', oracalCode: '049', category: 'blue' },
    'gentian': { hex: '#1A237E', oracalCode: '051', category: 'blue' },
    'midnight blue': { hex: '#191970', oracalCode: null, category: 'blue' },
    'steel blue': { hex: '#4682B4', oracalCode: null, category: 'blue' },
    'powder blue': { hex: '#B0E0E6', oracalCode: null, category: 'blue' },

    // Greens
    'green': { hex: '#00B050', oracalCode: '080', category: 'green' },
    'light green': { hex: '#90EE90', oracalCode: '081', category: 'green' },
    'dark green': { hex: '#006400', oracalCode: '082', category: 'green' },
    'forest green': { hex: '#228B22', oracalCode: '083', category: 'green' },
    'lime': { hex: '#32CD32', oracalCode: '084', category: 'green' },
    'lime green': { hex: '#32CD32', oracalCode: '084', category: 'green' },
    'emerald': { hex: '#50C878', oracalCode: '085', category: 'green' },
    'emerald green': { hex: '#50C878', oracalCode: '085', category: 'green' },
    'olive': { hex: '#808000', oracalCode: '086', category: 'green' },
    'olive green': { hex: '#808000', oracalCode: '086', category: 'green' },
    'mint': { hex: '#98FF98', oracalCode: '055', category: 'green' },
    'mint green': { hex: '#98FF98', oracalCode: '055', category: 'green' },
    'sea green': { hex: '#2E8B57', oracalCode: null, category: 'green' },
    'hunter green': { hex: '#355E3B', oracalCode: null, category: 'green' },
    'sage': { hex: '#9DC183', oracalCode: null, category: 'green' },
    'grass green': { hex: '#7CFC00', oracalCode: null, category: 'green' },
    'kelly green': { hex: '#4CBB17', oracalCode: null, category: 'green' },
    'moss green': { hex: '#8A9A5B', oracalCode: null, category: 'green' },
    'jade': { hex: '#00A86B', oracalCode: null, category: 'green' },
    'pine green': { hex: '#01796F', oracalCode: null, category: 'green' },
    'signal green': { hex: '#00B050', oracalCode: '080', category: 'green' },

    // Browns
    'brown': { hex: '#8B4513', oracalCode: '083', category: 'brown' },
    'light brown': { hex: '#CD853F', oracalCode: null, category: 'brown' },
    'dark brown': { hex: '#3D2314', oracalCode: null, category: 'brown' },
    'tan': { hex: '#D2B48C', oracalCode: null, category: 'brown' },
    'chocolate': { hex: '#7B3F00', oracalCode: null, category: 'brown' },
    'coffee': { hex: '#6F4E37', oracalCode: null, category: 'brown' },
    'caramel': { hex: '#FFD59A', oracalCode: null, category: 'brown' },
    'camel': { hex: '#C19A6B', oracalCode: null, category: 'brown' },
    'cocoa': { hex: '#D2691E', oracalCode: null, category: 'brown' },
    'copper': { hex: '#B87333', oracalCode: null, category: 'brown' },
    'bronze': { hex: '#CD7F32', oracalCode: null, category: 'brown' },
    'rust': { hex: '#B7410E', oracalCode: null, category: 'brown' },
    'mahogany': { hex: '#C04000', oracalCode: null, category: 'brown' },
    'nut brown': { hex: '#6B4423', oracalCode: '083', category: 'brown' },
    'sand': { hex: '#C2B280', oracalCode: null, category: 'brown' },
    'khaki': { hex: '#F0E68C', oracalCode: null, category: 'brown' },

    // Metallics
    'metallic gold': { hex: '#D4AF37', oracalCode: '024', category: 'metallic', finish: 'metallic' },
    'metallic silver': { hex: '#C0C0C0', oracalCode: '076', category: 'metallic', finish: 'metallic' },
    'metallic bronze': { hex: '#CD7F32', oracalCode: null, category: 'metallic', finish: 'metallic' },
    'metallic copper': { hex: '#B87333', oracalCode: null, category: 'metallic', finish: 'metallic' },
    'chrome': { hex: '#E8E8E8', oracalCode: null, category: 'metallic', finish: 'chrome' },
    'mirror gold': { hex: '#FFD700', oracalCode: null, category: 'metallic', finish: 'mirror' },
    'mirror silver': { hex: '#C0C0C0', oracalCode: null, category: 'metallic', finish: 'mirror' },

    // Fluorescent/Neon
    'fluorescent yellow': { hex: '#CCFF00', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'neon yellow': { hex: '#CCFF00', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'fluorescent orange': { hex: '#FF6600', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'neon orange': { hex: '#FF6600', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'fluorescent red': { hex: '#FF3131', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'neon red': { hex: '#FF3131', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'fluorescent green': { hex: '#39FF14', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'neon green': { hex: '#39FF14', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'fluorescent pink': { hex: '#FF6EFF', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'neon pink': { hex: '#FF6EFF', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' },
    'dayglo orange': { hex: '#FF5F00', oracalCode: null, category: 'fluorescent', finish: 'fluorescent' }
  },

  // ============================================================
  // REFLECTIVE VINYL COLORS
  // Standard colors for reflective sheeting (engineering grade, high intensity)
  // ============================================================
  reflectiveColors: {
    'white': { hex: '#FFFFFF', description: 'White reflective' },
    'yellow': { hex: '#FFD700', description: 'Yellow reflective' },
    'red': { hex: '#E60012', description: 'Red reflective' },
    'orange': { hex: '#FF6600', description: 'Orange reflective' },
    'green': { hex: '#00B050', description: 'Green reflective' },
    'blue': { hex: '#0066CC', description: 'Blue reflective' },
    'fluorescent yellow': { hex: '#CCFF00', description: 'Fluorescent yellow reflective' },
    'fluorescent orange': { hex: '#FF6600', description: 'Fluorescent orange reflective' },
    'fluorescent lime': { hex: '#39FF14', description: 'Fluorescent lime reflective' },
    // Chevron patterns
    'chevron white red': { hex: 'linear-gradient(135deg, #FFFFFF 50%, #E60012 50%)', description: 'White and red chevron' },
    'chevron yellow red': { hex: 'linear-gradient(135deg, #FFD700 50%, #E60012 50%)', description: 'Yellow and red chevron' },
    'chevron white black': { hex: 'linear-gradient(135deg, #FFFFFF 50%, #1A1A1A 50%)', description: 'White and black chevron' },
    'chevron yellow black': { hex: 'linear-gradient(135deg, #FFD700 50%, #1A1A1A 50%)', description: 'Yellow and black chevron' },
    // Common combined names
    'white red': { hex: '#E60012', description: 'White/red combination', secondary: '#FFFFFF' },
    'yellow red': { hex: '#E60012', description: 'Yellow/red combination', secondary: '#FFD700' }
  },

  // ============================================================
  // COLOR SYNONYMS - Alternative names for the same color
  // ============================================================
  synonyms: {
    // White variations
    'pure white': 'white',
    'bright white': 'white',
    'snow white': 'white',
    'paper white': 'white',

    // Black variations
    'jet black': 'black',
    'matte black': 'black',
    'gloss black': 'black',

    // Gray variations
    'light grey': 'light gray',
    'dark grey': 'dark gray',
    'pewter': 'slate',
    'graphite': 'charcoal',

    // Blue variations
    'baby blue': 'light blue',
    'powder blue': 'light blue',
    'midnight': 'navy',
    'cornflower': 'azure',
    'electric blue': 'bright blue',

    // Red variations
    'fire engine red': 'bright red',
    'blood red': 'dark red',
    'cherry': 'cherry red',

    // Green variations
    'grass': 'grass green',
    'lime': 'lime green',
    'pine': 'forest green',

    // Yellow variations
    'canary': 'canary yellow',
    'buttercup': 'bright yellow',
    'dandelion': 'golden yellow',

    // Purple variations
    'royal purple': 'purple',
    'deep purple': 'dark purple',
    'ultra violet': 'violet',
    'electric purple': 'purple',

    // Pink variations
    'bubblegum': 'hot pink',
    'rose pink': 'rose',
    'blush pink': 'blush',

    // Orange variations
    'pumpkin': 'orange',
    'carrot': 'orange',

    // Brown variations
    'espresso': 'dark brown',
    'latte': 'light brown',
    'mocha': 'coffee',
    'chocolate brown': 'chocolate',
    'sienna': 'rust',
    'terracotta': 'rust',

    // Metallic synonyms
    'gold metallic': 'metallic gold',
    'silver metallic': 'metallic silver',
    'copper metallic': 'metallic copper',
    'bronze metallic': 'metallic bronze',

    // Common misspellings
    'grey': 'gray',
    'grey dark': 'dark grey',
    'grey light': 'light grey',
    'collared': 'colored',
    'matt black': 'matte black',
    'mate black': 'matte black',
    'mate': 'matte',
    'glos': 'gloss',
    'metalic': 'metallic',
    'florescent': 'fluorescent',
    'flourescent': 'fluorescent',
    'reflectve': 'reflective',
    'refelctive': 'reflective'
  },

  // ============================================================
  // COLOR PATTERNS - Regex patterns for parsing color names
  // ============================================================
  patterns: {
    // Pattern: [modifier] [color] or [color] [modifier]
    modifierFirst: /^(dark|light|bright|pale|deep|pastel|medium|vivid)\s+(.+)$/i,
    modifierLast: /^(.+?)\s+(dark|light|bright|pale|deep|pastel|medium|vivid|gloss|glossy|matte|matt|satin|metallic|chrome|mirror|pearl|fluorescent|neon|holographic|glitter|transparent|translucent)$/i,
    // Pattern: [color] [intensity] [finish] (e.g., "red dark matte")
    multiModifier: /^(.+?)\s+(dark|light|bright|pale|deep|pastel|medium)\s+(gloss|glossy|matte|matt|satin|metallic|chrome|mirror|pearl|fluorescent|neon)$/i,
    // Chevron patterns
    chevron: /^chevron\s+(.+)/i,
    // Combined colors
    combined: /^(white|yellow|red|blue|green|black)\s+(white|yellow|red|blue|green|black)$/i
  }
};

/**
 * Color Utility Functions
 */
const VinylColorUtils = {
  /**
   * Parse a color name and extract base color and modifiers
   * @param {string} colorInput - The color name to parse
   * @returns {{ baseColor: string, modifiers: string[], hex: string }}
   */
  parseColor(colorInput) {
    if (!colorInput) return { baseColor: null, modifiers: [], hex: '#9ca3af' };

    const input = colorInput.toLowerCase().trim();

    // Check for chevron pattern
    const chevronMatch = input.match(VINYL_COLOR_KNOWLEDGE.patterns.chevron);
    if (chevronMatch) {
      const chevronColors = VINYL_COLOR_KNOWLEDGE.reflectiveColors[`chevron ${chevronMatch[1]}`];
      if (chevronColors) {
        return {
          baseColor: `chevron ${chevronMatch[1]}`,
          modifiers: ['reflective'],
          hex: chevronColors.hex,
          isReflective: true
        };
      }
    }

    // Check for combined colors (e.g., "white red", "yellow red")
    const combinedMatch = input.match(VINYL_COLOR_KNOWLEDGE.patterns.combined);
    if (combinedMatch) {
      const combinedKey = `${combinedMatch[1]} ${combinedMatch[2]}`;
      const combinedColor = VINYL_COLOR_KNOWLEDGE.reflectiveColors[combinedKey];
      if (combinedColor) {
        return {
          baseColor: combinedKey,
          modifiers: ['reflective'],
          hex: combinedColor.hex,
          secondaryHex: combinedColor.secondary,
          isReflective: true
        };
      }
    }

    // Try multi-modifier pattern (e.g., "red dark matte")
    const multiMatch = input.match(VINYL_COLOR_KNOWLEDGE.patterns.multiModifier);
    if (multiMatch) {
      const baseColor = this.resolveColorName(multiMatch[1]);
      const intensityModifier = multiMatch[2];
      const finishModifier = multiMatch[3];
      const baseHex = this.getBaseHex(baseColor);

      return {
        baseColor: baseColor,
        modifiers: [intensityModifier, finishModifier],
        hex: this.adjustHex(baseHex, intensityModifier),
        finish: finishModifier
      };
    }

    // Try modifier-first pattern (e.g., "dark red")
    const modifierFirstMatch = input.match(VINYL_COLOR_KNOWLEDGE.patterns.modifierFirst);
    if (modifierFirstMatch) {
      const modifier = modifierFirstMatch[1];
      const baseColor = this.resolveColorName(modifierFirstMatch[2]);
      const baseHex = this.getBaseHex(baseColor);

      return {
        baseColor: baseColor,
        modifiers: [modifier],
        hex: this.adjustHex(baseHex, modifier)
      };
    }

    // Try modifier-last pattern (e.g., "red dark", "blue matte")
    const modifierLastMatch = input.match(VINYL_COLOR_KNOWLEDGE.patterns.modifierLast);
    if (modifierLastMatch) {
      const baseColor = this.resolveColorName(modifierLastMatch[1]);
      const modifier = modifierLastMatch[2];
      const baseHex = this.getBaseHex(baseColor);
      const modifierInfo = VINYL_COLOR_KNOWLEDGE.modifiers[modifier];

      // If it's a finish modifier, don't adjust the color
      if (modifierInfo && modifierInfo.finish) {
        return {
          baseColor: baseColor,
          modifiers: [modifier],
          hex: baseHex,
          finish: modifierInfo.finish
        };
      }

      return {
        baseColor: baseColor,
        modifiers: [modifier],
        hex: this.adjustHex(baseHex, modifier)
      };
    }

    // No modifiers found, just look up the base color
    const baseColor = this.resolveColorName(input);
    return {
      baseColor: baseColor,
      modifiers: [],
      hex: this.getBaseHex(baseColor)
    };
  },

  /**
   * Resolve a color name to its canonical form
   * @param {string} colorName - The color name to resolve
   * @returns {string} The canonical color name
   */
  resolveColorName(colorName) {
    const normalized = colorName.toLowerCase().trim();

    // Check synonyms first
    if (VINYL_COLOR_KNOWLEDGE.synonyms[normalized]) {
      return VINYL_COLOR_KNOWLEDGE.synonyms[normalized];
    }

    // Check if it's a direct base color
    if (VINYL_COLOR_KNOWLEDGE.baseColors[normalized]) {
      return normalized;
    }

    // Check for partial matches (fuzzy matching)
    const fuzzyMatch = this.fuzzyMatchColor(normalized);
    if (fuzzyMatch) {
      return fuzzyMatch;
    }

    return normalized; // Return as-is if not found
  },

  /**
   * Get the hex code for a base color
   * @param {string} colorName - The base color name
   * @returns {string} The hex color code
   */
  getBaseHex(colorName) {
    const baseColor = VINYL_COLOR_KNOWLEDGE.baseColors[colorName];
    if (baseColor) {
      return baseColor.hex;
    }

    // Check reflective colors
    const reflectiveColor = VINYL_COLOR_KNOWLEDGE.reflectiveColors[colorName];
    if (reflectiveColor) {
      return reflectiveColor.hex;
    }

    // Default gray
    return '#9ca3af';
  },

  /**
   * Adjust a hex color based on intensity modifier
   * @param {string} hex - The base hex color
   * @param {string} modifier - The intensity modifier (dark, light, etc.)
   * @returns {string} The adjusted hex color
   */
  adjustHex(hex, modifier) {
    const modifierInfo = VINYL_COLOR_KNOWLEDGE.modifiers[modifier];
    if (!modifierInfo || modifierInfo.adjust === undefined) {
      return hex;
    }

    const adjustment = modifierInfo.adjust;
    return this.adjustBrightness(hex, adjustment);
  },

  /**
   * Adjust the brightness of a hex color
   * @param {string} hex - The hex color
   * @param {number} percent - Percentage adjustment (-100 to 100)
   * @returns {string} The adjusted hex color
   */
  adjustBrightness(hex, percent) {
    // Remove # if present
    hex = hex.replace('#', '');

    // Parse RGB values
    let r = parseInt(hex.substring(0, 2), 16);
    let g = parseInt(hex.substring(2, 4), 16);
    let b = parseInt(hex.substring(4, 6), 16);

    // Adjust brightness
    if (percent > 0) {
      r = Math.min(255, r + Math.round((255 - r) * (percent / 100)));
      g = Math.min(255, g + Math.round((255 - g) * (percent / 100)));
      b = Math.min(255, b + Math.round((255 - b) * (percent / 100)));
    } else {
      r = Math.max(0, r + Math.round(r * (percent / 100)));
      g = Math.max(0, g + Math.round(g * (percent / 100)));
      b = Math.max(0, b + Math.round(b * (percent / 100)));
    }

    // Convert back to hex
    const toHex = (c) => {
      const hex = c.toString(16);
      return hex.length === 1 ? '0' + hex : hex;
    };

    return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
  },

  /**
   * Fuzzy match a color name
   * @param {string} input - The input color name
   * @returns {string|null} The matched color name or null
   */
  fuzzyMatchColor(input) {
    const allColors = Object.keys(VINYL_COLOR_KNOWLEDGE.baseColors);

    // Check for partial match (color contains input or input contains color)
    for (const color of allColors) {
      if (color.includes(input) || input.includes(color)) {
        return color;
      }
    }

    // Check for Levenshtein distance (for typos)
    const maxDistance = 2;
    for (const color of allColors) {
      if (this.levenshteinDistance(input, color) <= maxDistance) {
        return color;
      }
    }

    return null;
  },

  /**
   * Calculate Levenshtein distance between two strings
   * @param {string} a - First string
   * @param {string} b - Second string
   * @returns {number} The Levenshtein distance
   */
  levenshteinDistance(a, b) {
    const matrix = [];

    for (let i = 0; i <= b.length; i++) {
      matrix[i] = [i];
    }

    for (let j = 0; j <= a.length; j++) {
      matrix[0][j] = j;
    }

    for (let i = 1; i <= b.length; i++) {
      for (let j = 1; j <= a.length; j++) {
        if (b.charAt(i - 1) === a.charAt(j - 1)) {
          matrix[i][j] = matrix[i - 1][j - 1];
        } else {
          matrix[i][j] = Math.min(
            matrix[i - 1][j - 1] + 1,
            matrix[i][j - 1] + 1,
            matrix[i - 1][j] + 1
          );
        }
      }
    }

    return matrix[b.length][a.length];
  },

  /**
   * Get all available color suggestions
   * @param {string} filter - Optional filter string
   * @param {string} stickerType - The sticker type (colored, reflective)
   * @returns {Array<{name: string, hex: string, category: string}>}
   */
  getColorSuggestions(filter = '', stickerType = 'colored') {
    const suggestions = [];
    const filterLower = filter.toLowerCase();

    if (stickerType === 'reflective') {
      // Return reflective colors (exclude chevron patterns - those are products, not stock)
      for (const [name, info] of Object.entries(VINYL_COLOR_KNOWLEDGE.reflectiveColors)) {
        // Skip chevron patterns - they are products, not vinyl stock colors
        if (name.startsWith('chevron')) continue;
        if (!filter || name.includes(filterLower)) {
          suggestions.push({
            name: name.charAt(0).toUpperCase() + name.slice(1),
            hex: info.hex,
            category: 'reflective',
            description: info.description
          });
        }
      }
    } else {
      // Return colored vinyl colors
      for (const [name, info] of Object.entries(VINYL_COLOR_KNOWLEDGE.baseColors)) {
        if (!filter || name.includes(filterLower)) {
          suggestions.push({
            name: name.charAt(0).toUpperCase() + name.slice(1),
            hex: info.hex,
            category: info.category,
            oracalCode: info.oracalCode
          });
        }
      }

      // Add common color + modifier combinations
      const commonModifiers = ['dark', 'light', 'matte', 'gloss'];
      const commonColors = ['red', 'blue', 'green', 'yellow', 'black', 'white'];

      for (const color of commonColors) {
        for (const modifier of commonModifiers) {
          const combinedName = `${color} ${modifier}`;
          if (!filter || combinedName.includes(filterLower)) {
            const parsed = this.parseColor(combinedName);
            suggestions.push({
              name: combinedName.charAt(0).toUpperCase() + combinedName.slice(1),
              hex: parsed.hex,
              category: 'common'
            });
          }
        }
      }
    }

    // Sort by relevance (exact matches first, then partial matches)
    suggestions.sort((a, b) => {
      const aExact = a.name.toLowerCase() === filterLower;
      const bExact = b.name.toLowerCase() === filterLower;
      if (aExact && !bExact) return -1;
      if (!aExact && bExact) return 1;
      return a.name.localeCompare(b.name);
    });

    return suggestions.slice(0, 20); // Limit to 20 suggestions
  },

  /**
   * Check if a color name is valid
   * @param {string} colorName - The color name to validate
   * @param {string} stickerType - The sticker type
   * @returns {{ valid: boolean, suggestion?: string, hex?: string }}
   */
  validateColor(colorName, stickerType = 'colored') {
    if (!colorName || !colorName.trim()) {
      return { valid: false };
    }

    const parsed = this.parseColor(colorName);

    // Check if we found a valid base color
    if (stickerType === 'reflective') {
      const reflectiveColor = VINYL_COLOR_KNOWLEDGE.reflectiveColors[parsed.baseColor];
      if (reflectiveColor) {
        return { valid: true, hex: reflectiveColor.hex };
      }
    } else {
      const baseColor = VINYL_COLOR_KNOWLEDGE.baseColors[parsed.baseColor];
      if (baseColor) {
        return { valid: true, hex: parsed.hex };
      }
    }

    // Check if we made a fuzzy match
    const fuzzyMatch = this.fuzzyMatchColor(parsed.baseColor);
    if (fuzzyMatch) {
      return {
        valid: true,
        hex: this.getBaseHex(fuzzyMatch),
        suggestion: fuzzyMatch.charAt(0).toUpperCase() + fuzzyMatch.slice(1)
      };
    }

    // Color not recognized, but allow it with default gray
    return { valid: true, hex: '#9ca3af', unrecognized: true };
  }
};

// Export for use in other modules
window.VINYL_COLOR_KNOWLEDGE = VINYL_COLOR_KNOWLEDGE;
window.VinylColorUtils = VinylColorUtils;
