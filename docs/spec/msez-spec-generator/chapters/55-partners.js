const {
  chapterHeading, h2,
  p
} = require("../lib/primitives");

module.exports = function build_chapter55() {
  return [
    chapterHeading("Chapter 55: Partner Network"),

    // --- 55.1 Implementation Partners ---
    h2("55.1 Implementation Partners"),
    p("Implementation partners fall into three categories. Jurisdictional partners are government agencies and free zone authorities that deploy the MSEZ Stack as sovereign infrastructure, contributing lawpacks, regpacks, and licensepacks for their regulatory frameworks. Operational partners are corporate service providers, banks, and financial institutions that run Mass API integrations and process real transactions through the corridor network. Integration partners are technology firms that build on the MSEZ Stack API, extending the platform with industry-specific modules, custom compliance domains, and specialized credential types."),

    // --- 55.2 Technology Partners ---
    h2("55.2 Technology Partners"),
    p("Technology partner infrastructure spans five layers. Cloud infrastructure partners provide sovereign-compliant hosting with data residency guarantees required by jurisdictional deployments. Identity partners provide KYC/KYB verification services, biometric authentication, and government identity system integrations (NADRA in Pakistan, ICA in UAE). Payment partners provide banking rails, SWIFT connectivity, real-time payment system integration (SBP Raast in Pakistan), and foreign exchange services. Legal technology partners provide Akoma Ntoso legislative document processing, regulatory change monitoring, and automated legal analysis for lawpack maintenance. Security partners provide penetration testing, cryptographic audits, and compliance certification services required for sovereign deployment approval."),
  ];
};
