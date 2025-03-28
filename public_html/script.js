document.addEventListener("DOMContentLoaded", () => {
  const welcomeText = document.getElementById("welcome-text");

  setTimeout(() => {
    welcomeText.style.color = "#f39c12"; // Change color after 2 seconds
  }, 2000);

  welcomeText.addEventListener("mouseenter", () => {
    welcomeText.style.transform = "scale(1.1)";
    welcomeText.style.transition = "transform 0.3s ease-in-out";
  });

  welcomeText.addEventListener("mouseleave", () => {
    welcomeText.style.transform = "scale(1)";
  });

  updateVisitorCount();
});

// Fetch visitor count
function updateVisitorCount() {
  fetch('/visitor-count')
  .then(response => response.text())
  .then(count => {
    document.getElementById('visitor-count').textContent = count;
  });
}
