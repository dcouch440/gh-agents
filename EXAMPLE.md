System Prompt
You are SVGGenerator, a professional SVG designer and illustrator specializing in transforming hand-drawn character sketches into polished, professional-quality vector graphics. Your expertise lies in interpreting rough sketches, enhancing them with refined artistry, and producing scalable, standalone SVG code that captures the original's essence while elevating it to professional standards.

Behavioral guidelines: Accurately interpret the sketch's key elements such as shapes, facial features, pose, and overall character. Elevate the design by adding smooth vector paths, vibrant colors, gradients or shadows for depth, and balanced composition to ensure scalability and visual appeal. Maintain the whimsical, childlike joy of the original but refine it with crisp lines, proper proportions, and harmonious colors (e.g., yellow for petals, green for stems, expressive faces). Internally note key features like round heads, eyes, smiles, petals, or stems before designing. Mentally test the SVG for render quality across different sizes. Output only the complete, standalone <svg> code with appropriate viewBox for responsiveness—use semantic elements like path, circle, etc. Do not include any explanations, additional text, or file operations.

You are the only agent in this pipeline, receiving the hand-drawn sketch directly as input. Your output is the final product, consumed directly by the user.

When producing the SVG, ensure it is a complete code block ready for rendering. Example output format:
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <circle cx="50" cy="50" r="40" fill="yellow" />
  <!-- Additional paths and elements here -->
</svg>
Input
<context>
Create a professional SVG representation of the provided hand-drawn character sketch. Envision and produce a final product as if made by a professional illustrator. Output only the complete SVG code; no saving files or extra text.
</context>

<assignment>
Interpret the provided hand-drawn character sketch, enhance it to professional illustrator quality by adding refinements like smooth paths, colors, and depth, and output only the complete standalone SVG code that represents the character.
</assignment>

<board_context>
## Stroke Coordinates
{"canvas":[728,650],"strokes":[{"points":[[292,253],[258,261],[244,290],[242,318],[251,352],[252,378],[264,389],[299,397],[315,382],[334,342],[341,280],[338,258],[302,247],[293,250]]},{"points":[[274,275],[266,280],[266,288],[271,292],[271,277]]},{"points":[[303,277],[291,280],[291,291],[300,284],[296,275]]},{"points":[[263,335],[271,343],[286,345],[295,336],[296,321]]},{"points":[[280,399],[272,492],[276,582]]},{"points":[[249,354],[232,358],[221,338],[228,324],[235,320],[242,324]]},{"points":[[240,310],[222,298],[227,271],[238,267],[252,278]]},{"points":[[261,258],[259,238],[271,228],[284,236],[293,254]]},{"points":[[308,247],[318,227],[329,220],[343,232],[346,241],[335,250]]},{"points":[[336,272],[360,279],[354,306],[339,306]]},{"points":[[330,335],[350,362],[351,374],[345,382],[330,388],[318,377]]}]}

</board_context>
Output
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 600">
  <!-- Background -->
  <rect width="400" height="600" fill="#E8F4FD"/>

  <!-- Main flower head: large oval petal-like face -->
  <defs>
    <radialGradient id="headGradient" cx="50%" cy="40%">
      <stop offset="0%" stop-color="#FFF2CC"/>
      <stop offset="50%" stop-color="#FFD700"/>
      <stop offset="100%" stop-color="#FFA500"/>
    </radialGradient>
    <dropShadow id="shadow" x="2" y="4" stdDeviation="2" flood-opacity="0.3"/>
  </defs>
  <ellipse cx="200" cy="200" rx="90" ry="110" fill="url(#headGradient)" stroke="#FF8C00" stroke-width="3" stroke-linejoin="round"/>
  <use href="#shadow" xlink:href="#headGradient" x="200" y="200" rx="90" ry="110" filter="url(#dropShadow)"/>

  <!-- Eyes: two shiny ovals -->
  <g stroke="#333" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <ellipse cx="165" cy="170" rx="12" ry="18" fill="#FFF" stroke="#333"/>
    <ellipse cx="235" cy="170" rx="12" ry="18" fill="#FFF" stroke="#333"/>
    <circle cx="165" cy="165" r="6" fill="#4A90E2"/>
    <circle cx="235" cy="165" r="6" fill="#4A90E2"/>
    <circle cx="167" cy="167" r="2" fill="white"/>
    <circle cx="237" cy="167" r="2" fill="white"/>
  </g>

  <!-- Nose: small triangle -->
  <path d="M200 195 L195 205 L205 205 Z" fill="#FF9999" stroke="#CC6666" stroke-width="1.5"/>

  <!-- Mouth: happy curved smile -->
  <path d="M170 230 Q200 260 230 230" fill="none" stroke="#FF6666" stroke-width="4" stroke-linecap="round"/>

  <!-- Petals: 6 vibrant yellow petals around head -->
  <g stroke="#FF4500" stroke-width="2">
    <path d="M110 140 Q80 180 110 220 Q140 200 110 140" fill="#FFD700" opacity="0.9"/>
    <path d="M140 80 Q170 50 200 80 Q170 110 140 80" fill="#FFD700" opacity="0.9"/>
    <path d="M260 80 Q290 50 320 80 Q290 110 260 80" fill="#FFD700" opacity="0.9"/>
    <path d="M290 140 Q320 180 290 220 Q260 200 290 140" fill="#FFD700" opacity="0.9"/>
    <path d="M200 50 Q170 20 140 50 Q170 80 200 50" fill="#FFED4A" opacity="0.95"/>
    <path d="M200 280 Q170 310 140 280 Q170 250 200 280" fill="#FFED4A" opacity="0.95"/>
  </g>

  <!-- Arms: wavy, friendly arms -->
  <g fill="#FFDBAC" stroke="#DAA06D" stroke-width="3" stroke-linejoin="round" stroke-linecap="round">
    <!-- Left arm -->
    <path d="M130 240 Q100 260 90 290 Q110 310 140 300 Q150 280 130 240" opacity="0.95"/>
    <!-- Right arm -->
    <path d="M270 240 Q300 260 310 290 Q290 310 260 300 Q250 280 270 240" opacity="0.95"/>
    <!-- Hand details -->
    <ellipse cx="85" cy="295" rx="8" ry="6" fill="#FFDBAC"/>
    <ellipse cx="315" cy="295" rx="8" ry="6" fill="#FFDBAC"/>
  </g>

  <!-- Stem body: thick green stem -->
  <defs>
    <linearGradient id="stemGradient" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#90EE90"/>
      <stop offset="100%" stop-color="#228B22"/>
    </linearGradient>
  </defs>
  <path d="M170 310 L185 400 Q190 500 200 550 Q210 500 215 400 L230 310" fill="url(#stemGradient)" stroke="#006400" stroke-width="8" stroke-linejoin="round"/>

  <!-- Leaves: two green leaves on stem -->
  <g fill="#32CD32" stroke="#228B22" stroke-width="2.5" stroke-linejoin="round">
    <path d="M150 380 Q120 360 130 420 Q160 410 150 380" opacity="0.9"/>
    <ellipse cx="130" cy="395" rx="15" ry="8" fill="#32CD32"/>
    <path d="M250 380 Q280 360 270 420 Q240 410 250 380" opacity="0.9"/>
    <ellipse cx="270" cy="395" rx="15" ry="8" fill="#32CD32"/>
  </g>

  <!-- Subtle highlights and shine on head -->
  <ellipse cx="200" cy="140" rx="30" ry="20" fill="white" opacity="0.4"/>
</svg>