$(function() {
    $('button.start-button').on('click', function() {
        console.log('start')
        this.disabled = true;
        this.style.backgroundColor = "#d6d6d6";

        fetch("/setting/start", {method: 'POST'})
        .then(response => response.json())
        .then(data => {
            console.log(data)
            this.disabled = false;
            window.location.reload();
        })
        .catch(error => {
            console.error('Error:', error);
        });

    });

    $('button.stop-button').on('click', function() {
        console.log('stop')
        this.disabled = true;
        this.style.backgroundColor = "#d6d6d6";

        fetch("/setting/exit", {method: 'POST'})
        .then(response => response.json())
        .then(data => {
            console.log(data)
            this.disabled = false;
            window.location.reload();
        })
        .catch(error => {
            console.error('Error:', error);
        });
    });
})

function submitForm(event) {
    event.preventDefault(); // 阻止表单默认提交行为

    var form = document.querySelector("form");
    var input_interval = parseInt(form.interval.value, 10)
    const taskInterval = {
        interval: input_interval, 
      };
    fetch("/setting/change_interval", {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify(taskInterval),
        })
        .then(response => response.json())
        .then(data => {
            console.log(data)
            this.disabled = false;
            window.location.reload();
        })
        .catch(error => {
            console.error('Error:', error);
        });
  }

function getDaemonPid() {
  fetch('/setting/get_daemon_pid', {method: 'GET'})
      .then(response => response.json())
      .then(data => {
          const info_list = data.data;
          const infoDiv = document.getElementById('daemon-pid');
          var html_string = '定时任务pid: '
          if (info_list != null) {
              html_string += info_list;
          } else {
              html_string += "无";
          }
          infoDiv.innerHTML = html_string;
      })
      .catch(error => console.error('Error:', error));
}

getDaemonPid()

function modifyMaxActiveDownloads() {
    var nums = document.getElementById("max-download").value;

    fetch('/download/modify_max_active_downloads?nums=' + nums, {method: 'POST'})
    .then(response => response.json())
    .then(data => {
        console.log(data)
        this.disabled = false;
        window.location.reload();
    })
    .catch(error => {
        console.error('Error:', error);
    });   
}

function getMaxActiveDownloads() {
  fetch('/download/get_max_active_downloads', {method: 'GET'})
      .then(response => response.json())
      .then(data => {
          const info_list = data.data;
          const infoDiv = document.getElementById('max-download-input');
          var html_string = '<input type="number"'
          if (info_list != null) {
              html_string += 'value=' + info_list;
          } else {
              html_string += '';
          }
          html_string += ' id="max-download" style="width: 40px;">';
          infoDiv.innerHTML = html_string;
      })
      .catch(error => console.error('Error:', error));
}

function syncAnime() {
    fetch('/setting/load_fin_task', {method: 'GET'})
    .then(response => response.json())
    .then(data => {
        console.log(data)
        this.disabled = false;
        window.location.reload();
    })
    .catch(error => console.error('Error:', error));
}

getMaxActiveDownloads()