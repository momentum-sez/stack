<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0"
  xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:akn="http://docs.oasis-open.org/legaldocml/ns/akn/3.0">

  <xsl:output method="html" indent="yes" encoding="UTF-8"/>

  <xsl:template match="/">
    <html>
      <head>
        <meta charset="utf-8"/>
        <title>Rendered Legal Document</title>
        <style>
          body { font-family: sans-serif; margin: 2rem; }
          h1,h2,h3 { margin-top: 1.5rem; }
          .eid { color: #888; font-size: 0.8rem; }
        </style>
      </head>
      <body>
        <xsl:apply-templates select="akn:akomaNtoso/*"/>
      </body>
    </html>
  </xsl:template>

  <xsl:template match="akn:act|akn:regulation|akn:doc">
    <h1><xsl:value-of select="name()"/></h1>
    <xsl:apply-templates select="akn:preface|akn:body"/>
  </xsl:template>

  <xsl:template match="akn:preface">
    <div>
      <xsl:apply-templates/>
    </div>
  </xsl:template>

  <xsl:template match="akn:body">
    <div>
      <xsl:apply-templates/>
    </div>
  </xsl:template>

  <xsl:template match="akn:section|akn:article">
    <h2>
      <xsl:value-of select="akn:num"/> <xsl:text> </xsl:text>
      <xsl:value-of select="akn:heading"/>
      <xsl:if test="@eId">
        <span class="eid"><xsl:text> (</xsl:text><xsl:value-of select="@eId"/><xsl:text>)</xsl:text></span>
      </xsl:if>
    </h2>
    <xsl:apply-templates select="akn:content|akn:paragraph|akn:doc|akn:blockList|akn:p"/>
  </xsl:template>

  <xsl:template match="akn:p">
    <p>
      <xsl:if test="@eId">
        <span class="eid"><xsl:value-of select="@eId"/><xsl:text> </xsl:text></span>
      </xsl:if>
      <xsl:apply-templates/>
    </p>
  </xsl:template>

  <xsl:template match="akn:heading|akn:num">
    <xsl:apply-templates/>
  </xsl:template>

  <xsl:template match="text()">
    <xsl:value-of select="."/>
  </xsl:template>

</xsl:stylesheet>
