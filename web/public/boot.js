try {
  var ua = navigator.userAgent || "";
  var plat = navigator.platform || "";
  if (/linux|x11/i.test(ua) || /linux/i.test(plat)) {
    document.documentElement.dataset.platform = "linux";
  }
} catch (e) {}

try {
  var root = document.documentElement;
  var bgSolid = localStorage.getItem("vynl.themeBgSolid");
  var theme = localStorage.getItem("vynl.theme");
  if (bgSolid) root.style.setProperty("--bg-solid", bgSolid);
  if (theme) root.dataset.theme = theme;
} catch (e) {}
