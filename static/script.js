function ready() {
    const times = document.querySelectorAll('.timestamp > time');
    const options = {
        weekday: 'short',
        day: 'numeric',
        month: 'short',
        year: 'numeric',

        hour: 'numeric',
        minute: 'numeric',
        second: 'numeric',
    };
    const format = new Intl.DateTimeFormat('en-GB', options);
    times.forEach((t) => {
        const dt = new Date(t.dateTime);
        t.textContent = format.format(dt);
    })
}
