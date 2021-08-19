var apiData = {
	time: new Date()
}

const init = () => {
	fetch("/api")
	.then(res => res.json())
	.then(data => {
		apiData = { ...apiData, ...data }

		updateData()

		fetchLatest()
	})
	.catch(err => console.log(err))

	setInterval(() => callForUpdates(), 1000)
}

const callForUpdates = () => {
	apiData.time = new Date()

	if (apiData.time.getSeconds() == 5) {
		fetchLatest()
	}

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

const fetchLatest = () => {
	fetch("/api/latest")
	.then(res => res.json())
	.then(data => {
		apiData = { ...apiData, ...data }

		updateData(true)
	})
	.catch(err => console.log(err))
}

const updateData = (fullUpdate=false) => {
	$("#api-version").text(`API Version: ${apiData.version ? apiData.version : "Unavailable"}`)
	$("#time").text(timeToStr(apiData.time, "America/Chicago"))

	if(fullUpdate) {
		console.log(apiData)

		if(apiData.temp) {
			$("#temp_f").text(apiData.temp.temp_f.toFixed(1))
			$("#temp_c").text(apiData.temp.temp_c.toFixed(1))
			$("#humidity").text(apiData.temp.humidity.toFixed(1))
			$("#temp_last_update").text(timeToStr(new Date(apiData.temp.last_updated), "America/Chicago"))
		}

		if(apiData.wind && apiData.wind_dir) {
			$("#wind_mph").text(apiData.wind.mph.toFixed(1))
			$("#wind_kph").text(apiData.wind.kph.toFixed(1))
			$("#dir").text(apiData.wind_dir.label)
			$("#dir_deg").text(apiData.wind_dir.dir.toFixed(1))
			$("#wind_last_update").text(timeToStr(new Date(apiData.wind.last_updated), "America/Chicago"))
		}

		if(apiData.rain) {
			$("#rain_in").text(apiData.rain.amnt_in.toFixed(2))
			$("#rain_cm").text(apiData.rain.amnt_cm.toFixed(2))
			$("#rain_last_update").text(timeToStr(new Date(apiData.rain.last_updated), "America/Chicago"))
		}
	}
}

init()