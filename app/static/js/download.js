function getTorrentInfo() {
    fetch('/download/qb_download_progress', {method: 'GET'})
        .then(response => response.json())
        .then(data => {
            const infoList = data;
            const infoDiv = document.getElementById('torrentInfo');
            var htmlString = `
                <tr>
                    <th class="column-name">番名</th>
                    <th class="column-episode">集数</th>
                    <th class="column-progress">进度</th>
                    <th class="column-done">已完成</th>
                    <th class="column-speed">下载速度</th>
                    <th class="column-eta">剩余时间</th>
                    <th class="column-peers">用户</th>
                    <th class="column-seeds">做种数</th>
                    <th class="column-size">大小</th>
                    <th class="column-state">状态</th>
                    <th class="column-button"></td>
                </tr>`;
            if (infoList.length > 0) {
                infoList.forEach(function(info) {
                    const done = Math.round(parseFloat(info.qb_info.done.trim()));
                    htmlString += `
                        <tr>
                            <td class="column-name" title='${info.anime_ame}'>
                                <a href="/anime/detail/${info.mikan_id}">${info.anime_name}</a>
                            </td>
                            <td class="column-episode">${info.episode}</td>
                            <td class="column-progress">
                                <progress value=${done} max="100"></progress>
                            </td>
                            <td class="column-done">${info.qb_info.done}</td>
                            <td class="column-speed">${info.qb_info.download_speed}</td>
                            <td class="column-eta">${info.qb_info.eta}</td>
                            <td class="column-peers">${info.qb_info.peers}</td>
                            <td class="column-seeds">${info.qb_info.seeds}</td>
                            <td class="column-size">${info.qb_info.size}</td>
                            <td class="column-state">${info.qb_info.state}</td>
                            <td class="column-button">
                                <button class="task-button" id="resume" onclick="handleQbTask('${info.torrent_name}', 3)">恢复</button>
                                <button class="task-button" id="pause" onclick="handleQbTask('${info.torrent_name}', 2)">暂停</button>
                                <button class="task-button" id="delete" onclick="handleQbTask('${info.torrent_name}', 1)">删除</button>
                            </td>
                        </tr>`
                });
            }
            infoDiv.innerHTML = htmlString;
            setTimeout(getTorrentInfo, 2000);
        })
        .catch(error => console.error('Error:', error));
}

getTorrentInfo()

function handleQbTask(torrentName, executeType) {
    const data = {
        torrent_name: torrentName,
        execute_type: executeType
    }
    fetch("/download/qb_execute", {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(data)
    })
        .then(response => response.json())
        .then(data => {
            window.location.reload();
        })
        .catch(error => {
            console.error('Error:', error);
        });
}