const { chapterHeading, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixF() {
  return [
    chapterHeading("Appendix F: Mass API Endpoint Reference"),
    table(
      ["API", "Base URL", "Swagger"],
      [
        ["Organization Info", "organization-info.api.mass.inc", "https://organization-info.api.mass.inc/organization-info/swagger-ui/index.html"],
        ["Investment Info", "investment-info-production.herokuapp.com", "https://investment-info-production-4f3779c81425.herokuapp.com/investment-info/swagger-ui/index.html"],
        ["Treasury Info", "treasury-info.api.mass.inc", "https://treasury-info.api.mass.inc/treasury-info/swagger-ui/index.html"],
        ["Consent Info", "consent.api.mass.inc", "https://consent.api.mass.inc/consent-info/swagger-ui/index.html"],
        ["Templating Engine", "templating-engine-prod.herokuapp.com", "https://templating-engine-prod-5edc768c1f80.herokuapp.com/templating-engine/swagger-ui/index.html"],
      ],
      [2000, 3400, 3960]
    ),
    spacer(),
  ];
};
