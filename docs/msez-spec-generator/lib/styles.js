const C = require("./constants");

module.exports = {
  default: {
    document: {
      run: { font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK }
    },
    heading1: {
      run: { size: 36, bold: true, font: C.BODY_FONT, color: C.H1_COLOR },
      paragraph: { spacing: { before: 360, after: 200 }, outlineLevel: 0 }
    },
    heading2: {
      run: { size: 28, bold: true, font: C.BODY_FONT, color: C.H2_COLOR },
      paragraph: { spacing: { before: 280, after: 160 }, outlineLevel: 1 }
    },
    heading3: {
      run: { size: 24, bold: true, font: C.BODY_FONT, color: C.H3_COLOR },
      paragraph: { spacing: { before: 200, after: 120 }, outlineLevel: 2 }
    }
  },
  paragraphStyles: []
};
