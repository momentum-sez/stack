const C = require("./constants");

module.exports = {
  default: {
    document: {
      run: { font: C.BODY_FONT, size: C.BODY_SIZE, color: C.DARK },
      paragraph: { alignment: "both", widowControl: true }
    },
    heading1: {
      run: { size: 32, bold: false, font: C.BODY_FONT, color: C.H1_COLOR },
      paragraph: {
        spacing: { before: 360, after: 240 },
        keepNext: true,
        keepLines: true,
        outlineLevel: 0
      }
    },
    heading2: {
      run: { size: 26, bold: false, font: C.BODY_FONT, color: C.H2_COLOR },
      paragraph: {
        spacing: { before: 300, after: 180 },
        keepNext: true,
        keepLines: true,
        outlineLevel: 1
      }
    },
    heading3: {
      run: { size: 24, bold: true, font: C.BODY_FONT, color: C.H1_COLOR },
      paragraph: {
        spacing: { before: 240, after: 120 },
        keepNext: true,
        keepLines: true,
        outlineLevel: 2
      }
    }
  },
  paragraphStyles: []
};
