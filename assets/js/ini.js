(() => {
	function shim(prototype) {
		const getExtension = prototype.getExtension;
		prototype.getExtension = function (name) {
			if (name === "WEBGL_debug_renderer_info") {
				return null;
			}
			return getExtension.apply(this, arguments);
		};
	}
	if (typeof WebGLRenderingContext !== "undefined")
		shim(WebGLRenderingContext.prototype);
	if (typeof WebGL2RenderingContext !== "undefined")
		shim(WebGL2RenderingContext.prototype);
})();

export default function () {
	const progressContainer = document.getElementById("progress-container");
	const progressBar = document.getElementById("progress-bar");
	const progressText = document.getElementById("progress-text");

	return {
		onStart: () => {
			console.log("Loading started");
			if (progressContainer) progressContainer.style.display = "block";
		},
		onProgress: ({ current, total }) => {
			if (total > 0 && progressBar) {
				const percent = Math.round((current / total) * 100);
				progressBar.style.width = `${percent}%`;
				if (progressText) {
					progressText.innerText = `Loading... ${percent}%`;
				}
			}
		},
		onComplete: () => {
			console.log("Loading complete");
			if (progressText) {
				progressText.innerText = "Initializing...";
			}
		},
		onSuccess: () => {
			console.log("Loading success");
		},
		onFailure: (error) => {
			console.error("Loading failed", error);
			if (progressContainer) {
				progressContainer.style.display = "none";
			}
			if (progressText) {
				progressText.innerText = `Failed to load: ${error}`;
				progressText.style.color = "red";
			}
		},
	};
}
