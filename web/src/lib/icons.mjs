const paths = {
  book: "M3 4h6l3 2 3-2h6v15h-6l-3 2-3-2H3V4Zm9 2v15",
  shelf:
    "M4 3v18m16-18v18M4 11h16M4 19h16M7 5v6m4-6v6m4-6 2 6M7 13v6m5-6v6m4-6 1 6",
  trophy:
    "M7 3h10v7a5 5 0 0 1-10 0V3Zm0 2H3v3a4 4 0 0 0 4 4m10-7h4v3a4 4 0 0 1-4 4m-5 3v6m-4 0h8",
  heart: "M12 20 3.5 11.5a5 5 0 0 1 8.5-5 5 5 0 0 1 8.5 5L12 20Z",
  star: "m12 2 3 6 7 1-5 5 1 7-6-3-6 3 1-7-5-5 7-1Z",
  violet: "m12 2 2.5 7.5L22 12l-7.5 2.5L12 22l-2.5-7.5L2 12l7.5-2.5Z",
  shield: "m12 2 9 4v6c0 5-9 10-9 10S3 17 3 12V6l9-4Zm-5 9 3 3 7-7",
  quill: "m4 20 2-7L17 2l5 5L11 18l-7 2Zm2-7 5 5M3 22h18",
  refresh: "M20 8a8 8 0 1 0 0 8m0-14v6h-6",
  grid: "M3 3h7v7H3V3Zm11 0h7v7h-7V3ZM3 14h7v7H3v-7Zm11 0h7v7h-7v-7Z",
  list: "M8 5h13M8 12h13M8 19h13M3 5h1M3 12h1M3 19h1",
  people:
    "M8 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm-6 10v-3a6 6 0 0 1 12 0v3m2-17a4 4 0 0 1 0 8m2 3a5 5 0 0 1 4 5v1",
  chart: "M4 20V9m8 11V3m8 17v-7M2 22h20",
  compass: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20Zm4-16-2 8-8 4 2-8 8-4Z",
  settings: "M4 6h16M4 12h16M4 18h16M8 3v6m8 0v6m-6 0v6",
  user: "M12 12a5 5 0 1 0 0-10 5 5 0 0 0 0 10Zm-9 10v-2a9 9 0 0 1 18 0v2",
  lock: "M5 10h14v12H5V10Zm3 0V6a4 4 0 0 1 8 0v4m-4 5v3",
  search: "M10 17a7 7 0 1 0 0-14 7 7 0 0 0 0 14Zm5-2 7 7",
};
export function icon(name, label = "") {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("class", "icon");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.6");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", label ? "false" : "true");
  if (label) svg.setAttribute("aria-label", label);
  const path = document.createElementNS(svg.namespaceURI, "path");
  path.setAttribute("d", paths[name] || paths.book);
  svg.append(path);
  return svg;
}
