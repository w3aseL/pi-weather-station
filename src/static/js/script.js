var apiData = {
	time: new Date()
}

const init = () => {
	fetch("/api")
	.then(res => res.json())
	.then(data => {
		apiData = { ...apiData, ...data }

		updateData()
	})
	.catch(err => console.log(err))

	setInterval(() => callForUpdates(), 1000)
}

const callForUpdates = () => {
	apiData["time"] = new Date()

	updateData()
}

const timeToStr = (time, timezone) => {
	let options = {
		timeZone: timezone,
		year: 'numeric',
		month: 'numeric',
		day: 'numeric',
		hour: 'numeric',
		minute: 'numeric',
		second: 'numeric',
	  };

	return time.toLocaleString([], options)
}

const updateData = () => {
	$("#api-version").text(`API Version: ${apiData.version ? apiData.version : "Unavailable"}`)
	$("#time").text(timeToStr(apiData.time, "America/Chicago"))
}

init()